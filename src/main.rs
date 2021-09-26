use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::process::Command;

use clap::Clap;
use cpp_demangle::Symbol;
use defs::ProcessInfo;
use object::Object;
use process_ext::ProcessExt;
use std::string::ToString;
use tracing::{debug, warn};
use tracing_subscriber;

mod breakpoint;
mod cli;
mod defs;
mod error;
mod function;
mod process_ext;
mod ptrace_engine;
mod utils;

use crate::cli::FuncSource;
use crate::defs::Result;
use crate::defs::{DebuggerEngine, DebuggerStatus};
use crate::function::{get_functions, get_functions_dwarf};
use crate::utils::get_base_region;

#[derive(Clap)]
pub struct Opts {
    /// How to resolve the functions in the binary
    #[clap(short, long, default_value = "heuristic")]
    source: FuncSource,

    #[clap(short, long)]
    ignore: Option<regex::Regex>,

    #[clap(short, long)]
    only: Option<regex::Regex>,

    /// Path to the binary to be traced
    binary: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let opts: Opts = Opts::parse();

    let binary = Path::new(&opts.binary);

    let bin_data = std::fs::read(binary)?;
    let obj_file = object::File::parse(&*bin_data)?;

    let binary_is_relocatable = matches!(
        obj_file.kind(),
        object::ObjectKind::Dynamic | object::ObjectKind::Relocatable
    );

    let cmd = Command::new(binary);
    let (engine, last_process) = ptrace_engine::PtraceEngine::spawn(cmd)?;

    let maps = last_process.get_memory_maps()?;
    let base_region = get_base_region(&maps, binary.canonicalize()?.to_str().unwrap()).unwrap();
    debug!(?maps, ?binary_is_relocatable, ?base_region);
    let mut funcs = match opts.source {
        FuncSource::Heuristic => get_functions(&obj_file),
        FuncSource::Dwarf => get_functions_dwarf(binary.to_str().unwrap(), &obj_file)?,
    };
    debug!(?funcs);

    for mut func in funcs.iter_mut() {
        if binary_is_relocatable {
            func.prologue_end_addr = func.prologue_end_addr.map(|x| x + base_region.start);
            func.address += base_region.start;
        }
        func.name = match Symbol::new(&func.name).map(|op| op.to_string()) {
            Ok(name) => name,
            // rustc demangle will return the original if it cant parser
            Err(_) => rustc_demangle::demangle(&func.name).to_string(),
        };
    }
    debug!(?funcs);
    let to_keep = |name: &str| match (&opts.only, &opts.ignore) {
        (Some(only), Some(ignore)) => (only.is_match(name) && !ignore.is_match(name)),
        (Some(only), None) => only.is_match(name),
        (None, Some(ignore)) => !ignore.is_match(name),
        (None, None) => true,
    };

    // filter functions
    funcs.retain(|f| to_keep(&f.name));

    start_trace(engine, last_process, funcs)?;
    Ok(())
}

fn start_trace<E>(
    mut engine: E,
    mut last_process: E::Process,
    funcs: Vec<function::Function>,
) -> Result<()>
where
    E: DebuggerEngine,
    E::Process: ProcessInfo + Debug,
{
    let mut funcs_map = HashMap::new();
    let mut funcs_prologue_map = HashMap::new();

    for func in funcs.into_iter() {
        let bp_addr = if let Some(start) = func.prologue_end_addr {
            funcs_prologue_map.insert(start, func);
            start
        } else {
            let addr = func.address;
            funcs_map.insert(func.address, func);
            addr
        };
        debug!("breakpoint set at {}", bp_addr);
        engine.set_breakpoint(&mut last_process, bp_addr)?;
    }
    engine.cont(&mut last_process)?;
    let mut depth = 0;

    // TODO: this wait and cont thingy is kinda falky
    while let Ok(status) = engine.wait() {
        debug!(?status);
        match status {
            DebuggerStatus::BreakpointHit(process, address) => {
                last_process = process;
                if let Some(func) = funcs_map.get(&address) {
                    depth += 1;
                    let registers = last_process.get_registers().unwrap();
                    print_function(&last_process, func, depth)?;
                    let ret_addr = last_process.read_u64_at(registers.rsp)?;
                    if ret_addr > 1 {
                        // println!("{:0x}", ret_addr);
                        engine.set_breakpoint(&mut last_process, ret_addr)?;
                    }
                } else if let Some(func) = funcs_prologue_map.get(&address) {
                    depth += 1;
                    let registers = last_process.get_registers().unwrap();
                    print_function(&last_process, func, depth)?;
                    let base_ptr = last_process.read_u64_at(registers.rbp)?;
                    if base_ptr > 1 {
                        let ret_addr = last_process.read_u64_at(base_ptr + 8)?;
                        engine.set_breakpoint(&mut last_process, ret_addr)?;
                    }
                } else {
                    // TODO: this is the ret this should be better lol
                    let registers = last_process.get_registers().unwrap();
                    println!("{}{}", str::repeat("| ", depth), registers.rax);
                    depth -= 1;
                }
            }
            DebuggerStatus::Exited(_pid, _exit_code) => {
                break;
            }
            DebuggerStatus::Stopped(pid) => {
                warn!(?pid, "got Stopped event");
                break;
            }
            _ => {}
        }
        engine.cont(&mut last_process).unwrap();
    }
    Ok(())
}

fn print_function<P: ProcessInfo>(
    process: &P,
    func: &function::Function,
    depth: usize,
) -> Result<()> {
    let params = process.get_fn_param_values(&func.parameters)?;

    println!(
        "{}{}({})",
        str::repeat("| ", depth),
        func.name,
        params.join(", ")
    );
    Ok(())
}
