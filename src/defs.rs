use std::io;
use std::path::PathBuf;
use std::process::{Child, Command};

use crate::error;

pub trait ProcessMem {
    fn read_at(&self, address: u64, data: &mut [u8]) -> io::Result<usize>;
    fn write_at(&mut self, address: u64, data: &[u8]) -> io::Result<usize>;
}

pub type Result<T> = std::result::Result<T, error::Error>;

pub type Registers = nix::libc::user_regs_struct;

pub trait ProcessInfo {
    fn file_path(&self) -> io::Result<PathBuf>;
    fn get_registers(&self) -> Result<Registers>;
    fn set_registers(&self, regs: Registers) -> Result<()>;
    // TODO: maybe this should be somewhere else maybe in the engine?
    fn step(&self) -> Result<()>;
}

#[cfg(unix)]
pub type Pid = nix::unistd::Pid;

#[derive(Debug)]
pub enum DebuggerStatus {
    /// Breakpoint hit for the Pid at address u64
    BreakpointHit(Pid, u64),
    /// Stopeed for some reason
    // TODO: add reason
    Stopped(Pid),
    /// Exited(Pid, exit_code)
    Exited(Pid, i32),
    /// Some unknown status
    Unknown,
}

pub trait DebuggerEngine {
    fn spawn(cmd: Command) -> Result<(Self, Child)>
    where
        Self: Sized;
    fn set_breakpoint(&mut self, pid: Pid, address: u64) -> Result<()>;
    fn cont(&mut self, pid: Pid) -> Result<()>;
    // fn step(&mut self, pid: Pid) -> Result<()>;
    fn wait(&mut self) -> Result<DebuggerStatus>;
}
