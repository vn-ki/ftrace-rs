use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use defs::ProcessInfo;
use process_ext::ProcessExt;
use object::Object;
use ptrace_engine::Process;
use tracing::{debug, warn};
use tracing_subscriber;

mod breakpoint;
mod defs;
mod error;
mod function;
mod ptrace_engine;
mod utils;
mod process_ext;

use crate::defs::{DebuggerEngine, DebuggerStatus};
use crate::defs::{Pid, Result};
use crate::function::{get_functions, get_functions_dwarf};
use crate::utils::get_base_region;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let binary = Path::new("./fact-pie");
    let bin_data = std::fs::read(binary)?;
    let obj_file = object::File::parse(&*bin_data)?;
    let binary_is_relocatable = matches!(
        obj_file.kind(),
        object::ObjectKind::Dynamic | object::ObjectKind::Relocatable
    );
    let dwarf_funcs = get_functions_dwarf(binary.to_str().unwrap())?;
    debug!(?dwarf_funcs);

    let cmd = Command::new(binary);
    let (mut engine, mut last_process) = ptrace_engine::PtraceEngine::spawn(cmd)?;

    let funcs = get_functions(&obj_file);
    debug!(?funcs);

    let mut funcs_map = HashMap::new();

    let maps = last_process.get_memory_maps()?;
    debug!(?maps);
    let base_region = get_base_region(&maps, binary.canonicalize()?.to_str().unwrap()).unwrap();
    debug!(?binary_is_relocatable, ?base_region);

    for func in funcs.into_iter() {
        let bp_addr = if binary_is_relocatable {
            base_region.start + func.address
        } else {
            func.address
        };
        debug!("breakpoint set at {}", bp_addr);
        engine.set_breakpoint(&mut last_process, bp_addr)?;
        funcs_map.insert(bp_addr, func);
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
                    let params = last_process.get_fn_param_values(&func.parameters)?;
                    println!(
                        "{}{}({})",
                        str::repeat("| ", depth),
                        func.name,
                        params.join(", ")
                    );
                    let ret_addr = {
                        let mut ret_addr: [u8; 8] = [0; 8];
                        last_process.read_at(registers.rsp, &mut ret_addr)?;
                        u64::from_le_bytes(ret_addr)
                    };
                    if ret_addr > 1 {
                        // println!("{:0x}", ret_addr);
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
