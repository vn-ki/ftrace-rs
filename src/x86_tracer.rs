use std::collections::HashMap;

use tracing::debug;

use crate::breakpoint::Breakpoint;
use crate::defs::{ProcessInfo, ProcessMem, Tracer, Result};
use crate::obj_helper::get_functions;

pub struct X86Tracer {
    breakpoints: HashMap<u64, Breakpoint>,
}

impl X86Tracer {
    pub fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
        }
    }
}

impl<T: ProcessMem + ProcessInfo> Tracer<T> for X86Tracer {
    fn init(&mut self, process: &mut T) -> Result<()> {
        let file = process.file_path()?;
        debug!("reading file {:?}", file);
        let bin_data = std::fs::read(file)?;
        let obj_file = object::File::parse(&*bin_data)?;
        let funcs = get_functions(&obj_file);
        debug!(?funcs);

        for func in &funcs {
            debug!("breakpoint set at {}", func.address);
            let mut bp = Breakpoint::new(func.address);
            bp.enable(process)?;
            self.breakpoints.insert(func.address, bp);
        }
        Ok(())
    }

    fn breakpoint_hit(&mut self, process: &mut T) -> Result<()> {
        let regs = process.get_registers()?;
        debug!(?regs);
        let addr = regs.rip - 1;
        if let Some(bp) = self.breakpoints.get_mut(&addr) {
            // if let Some(func) = funcs.iter().find(|f| f.address == addr) {
            //     print_function(&tracee, func);
            // }
            // disable bp
            // reduce rip
            // step
            // re-enable bp
            bp.disable(process)?;
            let mut regs = process.get_registers()?;
            regs.rip -= 1;
            process.set_registers(regs)?;
            process.step()?;
            bp.enable(process)?;
        }
        Ok(())
    }
}
