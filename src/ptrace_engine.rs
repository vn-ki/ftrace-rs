//! nix ptrace engine
//!
//! Refs:
//! - A debugger using ptrace: https://blog.tartanllama.xyz/writing-a-linux-debugger-setup/
//! - Mac vmmap in Rust: https://jvns.ca/blog/2018/01/26/mac-memory-maps/

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use std::{os::unix::prelude::CommandExt, process::Command};

use nix::sys::ptrace;
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::os::unix::fs::FileExt;
use tracing::debug;

use crate::breakpoint::Breakpoint;
use crate::defs::{
    DebuggerEngine, DebuggerStatus, MemoryRegion, ProcessInfo, ProcessMem, Registers, Result,
};
use crate::utils::parse_address_without_0x;

pub struct PtraceEngine {
    breakpoints: HashMap<u64, Breakpoint>,
}

impl DebuggerEngine for PtraceEngine {
    fn set_breakpoint(&mut self, pid: Pid, address: u64) -> Result<()> {
        let process = &mut Process(pid);
        let mut bp = Breakpoint::new(address);
        bp.enable(process)?;
        self.breakpoints.insert(address, bp);
        Ok(())
    }

    fn cont(&mut self, pid: Pid) -> Result<()> {
        if let Some(bp) = self.get_breakpoint(pid)? {
            ptrace::step(pid, None)?;
            let _status = wait::waitpid(pid, None)?;
            bp.enable(&mut Process(pid))?;
        }
        ptrace::cont(pid, None)?;
        Ok(())
    }

    // fn step(&mut self, pid: Pid) -> Result<()> {
    //     if let Some(status) = self.step_over_bp(pid)? {
    //         return self.handle_wait(status);
    //     }
    //     Ok(())
    // }

    fn spawn(cmd: Command) -> Result<(Self, Child)> {
        let child = Self::spawn_cmd(cmd)?;
        Ok((
            Self {
                breakpoints: HashMap::new(),
            },
            child,
        ))
    }

    fn wait(&mut self) -> Result<DebuggerStatus> {
        // XXX: the issue with seperating wait from cont and wait
        // is that step and cont must be followed by wait
        self.handle_wait(wait::waitpid(None, Some(WaitPidFlag::__WALL))?)
    }
}

impl PtraceEngine {
    pub fn get_breakpoint(&mut self, pid: Pid) -> Result<Option<&mut Breakpoint>> {
        let regs = Process(pid).get_registers()?;
        Ok(self.breakpoints.get_mut(&regs.rip))
    }

    pub fn handle_wait(&mut self, status: WaitStatus) -> Result<DebuggerStatus> {
        use nix::sys::signal::Signal::*;
        match status {
            WaitStatus::Stopped(pid, SIGTRAP) => {
                // debug!(?status);
                let process = &mut Process(pid);
                let mut regs = process.get_registers()?;
                let bp_addr = regs.rip - Breakpoint::instr_len();

                if let Some(bp) = self.breakpoints.get_mut(&bp_addr) {
                    bp.disable(process)?;
                    regs.rip -= Breakpoint::instr_len();
                    process.set_registers(regs)?;
                    return Ok(DebuggerStatus::BreakpointHit(pid, bp_addr));
                }
                debug!(?regs, "did not find a breakpoint, still got a sigtrap");
                Ok(DebuggerStatus::Stopped(pid))
            }
            WaitStatus::Stopped(pid, SIGSEGV) => {
                debug!(?status);
                Ok(DebuggerStatus::Stopped(pid))
            }
            WaitStatus::Exited(pid, exit_code) => {
                debug!("process with pid {} exited with code {}", pid, exit_code);
                Ok(DebuggerStatus::Exited(pid, exit_code))
            }
            _ => {
                debug!(?status);
                Ok(DebuggerStatus::Unknown)
            }
        }
    }

    fn spawn_cmd(mut cmd: Command) -> Result<Child> {
        unsafe {
            cmd.pre_exec(|| ptrace::traceme().map_err(|errno| errno.into()));
        }
        let child = cmd.spawn()?;
        Ok(child)
    }
}

pub struct Process(pub Pid);

impl Process {
    fn proc_mem_path(&self) -> String {
        format!("/proc/{}/mem", self.0)
    }
    fn proc_cmdline_path(&self) -> String {
        format!("/proc/{}/cmdline", self.0)
    }
    fn proc_vmmaps(&self) -> String {
        format!("/proc/{}/maps", self.0)
    }

    #[tracing::instrument]
    fn vmmap_line_to_region(line: &str) -> MemoryRegion {
        // TODO: return result instead of unwrapping
        let mut iter = line.splitn(5, char::is_whitespace);
        let [range, _, _, _, file] = [(); 5].map(|_| iter.next().unwrap());
        debug!(?range, ?file);

        let mut range_iter = range.splitn(2, '-');
        let start = parse_address_without_0x(range_iter.next().unwrap()).unwrap();
        let end = parse_address_without_0x(range_iter.next().unwrap()).unwrap();
        let size = end - start;

        let mut iter = file.splitn(2, char::is_whitespace);
        // skip over the the first number thingy
        iter.next().unwrap();
        if let Some(file) = iter.next() {
            return MemoryRegion {
                filename: Some(file.trim().to_owned()),
                start,
                size,
            };
        }
        MemoryRegion {
            filename: None,
            start,
            size,
        }
    }
}

impl ProcessInfo for Process {
    fn file_path(&self) -> std::io::Result<PathBuf> {
        std::fs::read_to_string(self.proc_cmdline_path()).map(|s| {
            // TODO: does the cmdline always has \0 at the end??
            let nul_range_end = s.chars().position(|c| c == '\0').unwrap_or(s.len());
            s[0..nul_range_end].into()
        })
    }

    fn get_registers(&self) -> Result<Registers> {
        ptrace::getregs(self.0).map_err(|err| err.into())
    }

    fn set_registers(&self, regs: Registers) -> Result<()> {
        ptrace::setregs(self.0, regs).map_err(|err| err.into())
    }

    fn get_memory_maps(&self) -> Result<Vec<MemoryRegion>> {
        Ok(std::fs::read_to_string(self.proc_vmmaps())?
            .lines()
            .map(Self::vmmap_line_to_region)
            .collect())
    }
}

impl ProcessMem for Process {
    fn read_at(&self, address: u64, data: &mut [u8]) -> std::io::Result<usize> {
        std::fs::File::open(self.proc_mem_path())?.read_at(data, address)
    }

    fn write_at(&mut self, address: u64, data: &[u8]) -> std::io::Result<usize> {
        std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .open(self.proc_mem_path())?
            .write_at(data, address)
    }
}
