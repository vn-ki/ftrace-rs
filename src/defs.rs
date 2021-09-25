use std::io;
use std::path::PathBuf;
use std::process::Command;

use crate::error;

pub use gimli::Register;

// TODO: this and Register both is pretty confusing namingwise
pub type Registers = nix::libc::user_regs_struct;

pub type Result<T> = std::result::Result<T, error::Error>;

#[derive(Debug)]
pub struct MemoryRegion {
    pub filename: Option<String>,
    pub start: u64,
    pub size: u64,
}

/// This is an abstraction over getting specific process related into on target systems
pub trait ProcessInfo {
    /// Returns the file path of the process
    fn file_path(&self) -> io::Result<PathBuf>;

    fn get_memory_maps(&self) -> Result<Vec<MemoryRegion>>;

    /// Gets the registers of the process
    // TODO: should this be DebuggerEngine?
    fn get_registers(&self) -> Result<Registers>;

    /// Sets the registers of the process
    // TODO: should this be DebuggerEngine?
    fn set_registers(&self, regs: Registers) -> Result<()>;

    /// Read memory at address
    fn read_at(&self, address: u64, data: &mut [u8]) -> io::Result<usize>;

    /// Write memory at address
    fn write_at(&mut self, address: u64, data: &[u8]) -> io::Result<usize>;
}

#[derive(Debug)]
pub enum DebuggerStatus<P: ProcessInfo> {
    /// Breakpoint hit for the Pid at address u64
    BreakpointHit(P, u64),
    /// Stopeed for some reason
    // TODO: add reason
    Stopped(P),
    /// Exited(Pid, exit_code)
    Exited(P, i32),
    /// Some unknown status
    Unknown,
}

pub trait DebuggerEngine {
    type Process;

    fn spawn(cmd: Command) -> Result<(Self, Self::Process)>
    where
        Self: Sized;
    fn set_breakpoint(&mut self, pid: &mut Self::Process, address: u64) -> Result<()>;
    fn cont(&mut self, pid: &mut Self::Process) -> Result<()>;
    // fn step(&mut self, pid: Pid) -> Result<()>;
    fn wait(&mut self) -> Result<DebuggerStatus<Self::Process>>
    where
        Self::Process: ProcessInfo;
}
