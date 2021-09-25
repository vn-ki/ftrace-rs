mod dwarf;
mod heuristic;

use crate::defs::Register;
use crate::error::ParamFindingFailure;

pub use dwarf::{get_functions_dwarf, dwarf_get_line_breakpoints};
pub use heuristic::get_functions;

#[derive(Debug)]
pub struct Function {
    pub address: u64,
    pub prologue_end_addr: Option<u64>,
    pub name: String,
    pub parameters: Vec<std::result::Result<FormalParameter, ParamFindingFailure>>,
    pub return_type: Option<FormalParameterKind>,
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
    pub name: Option<String>,
    pub kind: FormalParameterKind,
    pub ty: Option<TypeKind>,
}

#[derive(Debug)]
// TODO: rename FormalParameterKind to SourceKind
pub enum FormalParameterKind {
    /// Parameter is stored in memory
    Memory(MemoryParam),
    /// Parameter is stored in registers
    Register(Register),
    // TODO: structs passed by value will have the values in multiple regs
}

#[derive(Debug)]
pub enum TypeKind {
    Void,
    BaseType(BaseType),
}

#[derive(Debug)]
pub struct BaseType {
    pub size: u64,
    pub encoding: BaseTypeEncoding,
    // TODO: add endianess and encoding
}

#[derive(Debug)]
pub enum BaseTypeEncoding {
    Address,
    Unsigned,
}
