mod dwarf;
mod heuristic;

use crate::defs::Register;
use crate::error::ParamFindingFailure;

pub use dwarf::get_functions_dwarf;
pub use heuristic::get_functions;

#[derive(Debug)]
pub struct Function {
    pub address: u64,
    pub name: String,
    pub parameters: Vec<std::result::Result<FormalParameter, ParamFindingFailure>>,
}

#[derive(Debug)]
pub struct MemoryParam {
    /// offset from base ptr
    // TODO: can this be something other than base ptr?
    pub offset: i64,
    /// size of param in bytes
    pub size: u64,
}

#[derive(Debug)]
pub struct FormalParameter {
    name: Option<String>,
    kind: FormalParameterKind,
}

#[derive(Debug)]
pub enum FormalParameterKind {
    /// Parameter is stored in memory
    Memory(MemoryParam),
    /// Parameter is stored in registers
    Register(Register),
    // TODO: structs passed by value will have the values in multiple regs
}
