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

    fn read_at_bytes(&self, addr: u64, n_bytes: usize) -> Result<Vec<u8>>;
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
                        Memory(mem) => {
                            let bp = self
                                .read_at_bytes(
                                    // XXX: TODO: HACK: this +16 is plain wrong
                                    (registers.rbp as i64 + 16 + mem.offset) as u64,
                                    mem.size as usize,
                                )
                                .map(|v| format_data(&v, mem.size));
                            format!("{:?}", bp)
                        }
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

    fn read_at_bytes(&self, addr: u64, n_bytes: usize) -> Result<Vec<u8>> {
        // TODO: use smallvec
        let mut v = vec![0; n_bytes];
        self.read_at(addr, &mut v[..])?;
        Ok(v)
    }
}

fn format_data(data: &[u8], size: u64) -> String {
    match size {
        8 => format!("{}", u64::from_le_bytes(data[0..8].try_into().unwrap())),
        4 => format!("{}", u32::from_le_bytes(data[0..4].try_into().unwrap())),
        2 => format!("{}", u16::from_le_bytes(data[0..2].try_into().unwrap())),
        _ => "not yet implemenented".to_string(),
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
