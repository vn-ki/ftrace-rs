use std::process::Command;
use std::collections::HashMap;

use defs::ProcessInfo;
use ptrace_engine::Process;
use tracing_subscriber;
use tracing::{debug, warn};

mod breakpoint;
mod defs;
mod error;
mod obj_helper;
mod ptrace_engine;
mod utils;

use crate::defs::{Pid, Result};
use crate::defs::{DebuggerEngine, DebuggerStatus};
use crate::obj_helper::get_functions;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cmd = Command::new("./fact");
    let (mut engine, child) = ptrace_engine::PtraceEngine::spawn(cmd)?;

    let bin_data = std::fs::read("./fact")?;
    let obj_file = object::File::parse(&*bin_data)?;
    let funcs = get_functions(&obj_file);
    // debug!(?funcs);
    let mut global_pid = Pid::from_raw(child.id() as i32);
    let mut funcs_map = HashMap::new();
    let process = &mut Process(global_pid);
    let maps = process.get_memory_maps()?;
    debug!(?maps);

    for func in funcs.into_iter() {
        debug!("breakpoint set at {}", func.address);
        engine.set_breakpoint(global_pid, func.address)?;
        funcs_map.insert(func.address, func);
    }
    engine.cont(global_pid)?;

    // TODO: this wait and cont thingy is kinda falky
    while let Ok(status) = engine.wait() {
        match status {
            DebuggerStatus::BreakpointHit(pid, address) => {
                debug!(?status);
                global_pid = pid;
                if let Some(func) = funcs_map.get(&address) {
                    println!("{}", func.name);
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

// type FunctionArgs = Vec<u64>;
//
// fn get_function_args(process: &Process, n_args: usize) -> FunctionArgs {
//     let regs = process.get_registers().unwrap();
//     // system-V abi
//     // RDI, RSI, RDX, RCX, R8, R9, [XYZ]MM0–7
//     let mut args = Vec::with_capacity(n_args);
//     let reg_args = &[regs.rdi, regs.rsi, regs.rdx, regs.rcx, regs.r8, regs.r9];
//     args.extend_from_slice(&reg_args[0..n_args]);
//     args
// }
