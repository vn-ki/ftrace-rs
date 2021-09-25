use crate::function::{FormalParameter, FormalParameterKind};
use crate::{
    defs::{ProcessInfo, Register, Registers, Result},
    error::ParamFindingFailure,
};

/// Generic helper functions for getting values from process
pub trait ProcessExt {
    fn get_fn_param_values(
        &self,
        params: &[std::result::Result<FormalParameter, ParamFindingFailure>],
    ) -> Result<Vec<String>>;

    // TODO: can i make u64 generic?
    fn read_u64_at(&self, addr: u64) -> Result<u64>;
}

impl<T: ProcessInfo> ProcessExt for T {
    fn get_fn_param_values(
        &self,
        params: &[std::result::Result<FormalParameter, ParamFindingFailure>],
    ) -> Result<Vec<String>> {
        let registers = self.get_registers()?;
        Ok(params
            .iter()
            .map(|param| match param {
                Ok(param) => {
                    use FormalParameterKind::*;
                    match param.kind {
                        Register(reg) => format!("{}", get_register(registers, reg)),
                        Memory(_) => format!("memory not implemented"),
                    }
                }
                Err(_) => "err".to_string(),
            })
            .collect())
    }

    fn read_u64_at(&self, addr: u64) -> Result<u64> {
        let mut ret_addr: [u8; 8] = [0; 8];
        self.read_at(addr, &mut ret_addr)?;
        Ok(u64::from_le_bytes(ret_addr))
    }
}

fn get_register(registers: Registers, register: Register) -> u64 {
    match register {
        gimli::X86_64::RDI => registers.rdi,
        gimli::X86_64::RSI => registers.rsi,
        gimli::X86_64::RDX => registers.rdx,
        gimli::X86_64::RCX => registers.rcx,
        _ => {
            panic!("register not found")
        }
    }
}
