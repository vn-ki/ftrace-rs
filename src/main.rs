use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use defs::ProcessInfo;
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
    let (mut engine, child) = ptrace_engine::PtraceEngine::spawn(cmd)?;

    let funcs = get_functions(&obj_file);
    debug!(?funcs);

    let mut global_pid = Pid::from_raw(child.id() as i32);
    let mut funcs_map = HashMap::new();

    let process = &mut Process(global_pid);
    let maps = process.get_memory_maps()?;
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
        engine.set_breakpoint(global_pid, bp_addr)?;
        funcs_map.insert(bp_addr, func);
    }
    engine.cont(global_pid)?;

    // TODO: this wait and cont thingy is kinda falky
    while let Ok(status) = engine.wait() {
        match status {
            DebuggerStatus::BreakpointHit(pid, address) => {
                debug!(?status);
                global_pid = pid;
                if let Some(func) = funcs_map.get(&address) {
                    println!("{} {:?}", func.name, func.parameters);
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
        engine.cont(global_pid).unwrap();
    }
    Ok(())
}
