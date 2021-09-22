use crate::function::{FormalParameter, FormalParameterKind};
use crate::{
    defs::{ProcessInfo, ProcessMem, Register, Registers, Result},
    error::ParamFindingFailure,
};

/// Generic helper functions for getting values from process
pub trait ProcessExt {
    fn get_fn_param_values(
        &self,
        params: &[std::result::Result<FormalParameter, ParamFindingFailure>],
    ) -> Result<Vec<String>>;
}

impl<T: ProcessMem + ProcessInfo> ProcessExt for T {
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
