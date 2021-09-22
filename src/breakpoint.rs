use crate::defs::{Result, ProcessInfo};

#[derive(Debug)]
pub struct Breakpoint {
    /// address of the breakpoint
    address: u64,
    /// the old data that was at the bp address
    old_data: u8,
}


impl<'a> Breakpoint {
    pub fn new(address: u64) -> Self {
        Self { address, old_data: 0, }
    }

    pub fn enable<T: ProcessInfo>(&mut self, tracee: &'a mut T) -> Result<()> {
        let mut mem: [u8; 1] = [0];
        tracee.read_at(self.address, &mut mem)?;
        self.old_data = mem[0];
        tracee.write_at(self.address, &[0xcc])?;
        Ok(())
    }

    pub fn disable<T: ProcessInfo>(&mut self, tracee: &'a mut T) -> Result<()> {
        tracee.write_at(self.address, &[self.old_data])?;
        Ok(())
    }

    pub const fn instr_len() -> u64 {
        1
    }
}
