use std::io;
use std::path::PathBuf;

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

pub trait Tracer<T: ProcessMem + ProcessInfo> {
    fn init(&mut self, process: &mut T) -> Result<()>;
    fn breakpoint_hit(&mut self, process: &mut T) -> Result<()>;
}
