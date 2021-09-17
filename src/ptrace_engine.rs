//! nix ptrace engine
//! Refs:
//! - https://blog.tartanllama.xyz/writing-a-linux-debugger-setup/

use std::path::PathBuf;
use std::process::Child;
use std::{os::unix::prelude::CommandExt, process::Command};

use nix::sys::ptrace;
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::os::unix::fs::FileExt;
use tracing::{debug, warn};

use crate::defs::{ProcessInfo, ProcessMem, Registers, Result, Tracer};

pub struct PtraceEngine;

impl PtraceEngine {
    pub fn start<T: Tracer<Process>>(cmd: Command, mut tracer: T) -> Result<()> {
        use nix::sys::signal::Signal::*;

        let child = Self::spawn(cmd)?;
        let pid = Pid::from_raw(child.id() as i32);
        if let Ok(WaitStatus::Stopped(_pid, SIGTRAP)) = wait::waitpid(pid, None) {
            // call init
            debug!("ptrace successful");
            tracer.init(&mut Process(pid))?;
            ptrace::cont(pid, None)?;
        } else {
            panic!("fix me");
        }

        while let Ok(status) = wait::waitpid(None, Some(WaitPidFlag::__WALL)) {
            match status {
                // TODO: call init when a new child appears
                WaitStatus::Stopped(pid, SIGTRAP) => {
                    debug!(?status);
                    if let Err(err) = tracer.breakpoint_hit(&mut Process(pid)) {
                        warn!(?err);
                    }
                }
                WaitStatus::Stopped(_pid, SIGSEGV) => {
                    debug!(?status);
                    break;
                }
                WaitStatus::Exited(pid, exit_code) => {
                    debug!("process with pid {} exited with code {}", pid, exit_code);
                    break;
                }
                _ => {
                    debug!(?status);
                }
            }
            ptrace::cont(pid, None)?;
        }
        Ok(())
    }

    fn spawn(mut cmd: Command) -> Result<Child> {
        unsafe {
            cmd.pre_exec(|| ptrace::traceme().map_err(|errno| errno.into()));
        }
        let child = cmd.spawn()?;
        Ok(child)
    }
}

pub struct Process(Pid);

impl Process {
    fn proc_mem_path(&self) -> String {
        format!("/proc/{}/mem", self.0)
    }
    fn proc_cmdline_path(&self) -> String {
        format!("/proc/{}/cmdline", self.0)
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

    fn get_registers(&self) -> crate::defs::Result<Registers> {
        ptrace::getregs(self.0).map_err(|err| err.into())
    }

    fn set_registers(&self, regs: Registers) -> crate::defs::Result<()> {
        ptrace::setregs(self.0, regs).map_err(|err| err.into())
    }

    fn step(&self) -> crate::defs::Result<()> {
        use nix::sys::signal::Signal::*;
        ptrace::step(self.0, None)?;
        if let Ok(WaitStatus::Stopped(_pid, SIGTRAP)) = wait::waitpid(self.0, None) {
            // call init
            debug!("stepped successfully");
        } else {
            panic!("fix me");
        }
        Ok(())
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
