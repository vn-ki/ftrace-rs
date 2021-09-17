use x86_tracer::X86Tracer;
use std::process::Command;
use tracing_subscriber;

mod breakpoint;
mod ptrace_engine;
mod defs;
mod x86_tracer;
mod error;
mod obj_helper;

use crate::defs::Result;

// fn parse_address(s: &str) -> Result<u64> {
//     let s = s.trim_start_matches("0x");
//     Ok(u64::from_str_radix(s, 16)?)
// }

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cmd = Command::new("./fact");
    ptrace_engine::PtraceEngine::start(cmd, X86Tracer::new())?;
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
