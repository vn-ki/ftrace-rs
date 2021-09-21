use std::collections::HashSet;
use std::result;

use ddbug_parser::FileHash;
use object::{read::ObjectSymbol, read::SymbolSection, Object, ObjectSection, SymbolKind};
use tracing::debug;

use crate::defs::Register;
use crate::error::ParamFindingFailure;

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

pub fn get_functions<'a>(obj: &'a object::File) -> Vec<Function> {
    let text_section_idx = obj.section_by_name(".text").unwrap().index();
    let mut funcs = vec![];
    for symbol in obj.symbols() {
        if matches!(symbol.kind(), SymbolKind::Text) {
            match symbol.section() {
                SymbolSection::Section(idx) if idx == text_section_idx => {
                    let func_name = symbol.name().unwrap();
                    debug!(?func_name, "parsing fn params");
                    let params = get_fn_params_heuristic(obj, &symbol).unwrap();
                    funcs.push(Function {
                        address: symbol.address(),
                        // TODO: fix this
                        name: func_name.into(),
                        parameters: params,
                    });
                }
                _ => {}
            }
        }
    }
    funcs
}

fn get_fn_params_heuristic<'a, T>(
    obj: &'a object::File,
    symbol: &T,
) -> crate::defs::Result<Vec<result::Result<FormalParameter, ParamFindingFailure>>>
where
    T: object::read::ObjectSymbol<'a>,
{
    use capstone::prelude::*;

    let code = obj
        .section_by_index(symbol.section_index().unwrap())
        .unwrap()
        .data_range(symbol.address(), symbol.size())
        .unwrap()
        .unwrap();

    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .syntax(arch::x86::ArchSyntax::Intel)
        .detail(true)
        .build()
        .expect("Failed to create Capstone object");

    let insns = cs.disasm_all(code, 0x1000).expect("Failed to disassemble");

    let mut regs_written: HashSet<RegId> = HashSet::new();
    regs_written.insert(capstone::arch::x86::X86Reg::X86_REG_RSP.into());
    regs_written.insert(capstone::arch::x86::X86Reg::X86_REG_RIP.into());
    regs_written.insert(capstone::arch::x86::X86Reg::X86_REG_RBP.into());
    let mut arg_regs = HashSet::new();

    for instr in insns.iter() {
        let detail: InsnDetail = cs.insn_detail(&instr).expect("Failed to get insn detail");

        for reg in detail.regs_read() {
            if let None = regs_written.get(reg) {
                arg_regs.insert(reg.to_owned());
            }
        }
        for reg in detail.regs_write() {
            regs_written.insert(reg.to_owned());
        }
    }

    fn cs_reg_to_register(reg: &RegId) -> Register {
        Register(reg.0)
    }
    let args: Vec<_> = arg_regs
        .iter()
        .map(|reg| Ok(FormalParameter {
            name: None,
            kind: FormalParameterKind::Register(cs_reg_to_register(reg)),
        }))
        .collect();
    debug!(?args);

    Ok(args)
}

pub fn get_functions_dwarf(filename: &str) -> crate::defs::Result<Vec<Function>> {
    let mut funcs = Vec::new();
    ddbug_parser::File::parse(filename, |file| {
        let file_hash = FileHash::new(file);
        for unit in file.units() {
            for function in unit.functions() {
                if let Some(name) = function.name() {
                    let details = function.details(&file_hash);
                    let params = details
                        .parameters()
                        .into_iter()
                        .map(|p| parse_dwarf_param(p, &file_hash))
                        .collect::<Vec<_>>();
                    if let Some(address) = function.address() {
                        funcs.push(Function {
                            name: name.to_string(),
                            parameters: params,
                            address,
                        })
                    }
                }
            }
        }
        Ok(())
    })?;

    Ok(funcs)
}

fn parse_dwarf_param(
    param: &ddbug_parser::Parameter,
    file: &ddbug_parser::FileHash,
) -> std::result::Result<FormalParameter, ParamFindingFailure> {
    let regs: Vec<_> = param.registers().collect();
    if regs.len() != 0 {
        debug!(?regs);
        // TODO: len > 1: struct by value?
        return Ok(FormalParameter {
            name: param.name().map(|s| s.to_string()),
            kind: FormalParameterKind::Register(Register(regs[0].1 .0)),
        });
    }
    let fl: Vec<_> = param.frame_locations().collect();
    if fl.len() != 0 {
        debug!(?fl);
        // TODO: investigate when len > 1
        if let Some(size) = param.byte_size(file) {
            return Ok(FormalParameter {
                name: param.name().map(|s| s.to_string()),
                kind: FormalParameterKind::Memory(MemoryParam {
                    offset: fl[0].offset,
                    size,
                }),
            });
        }
        return Err(ParamFindingFailure::DwarfNoSize);
    }
    Err(ParamFindingFailure::DwarfNoFrameLocNoReg)
}
