use gimli::{AttributeValue, DW_AT_high_pc, DW_AT_low_pc, DW_AT_name, DebugInfo};
use object::{Object, ObjectSection};
use tracing::debug;

use crate::function::{
    FormalParameter, FormalParameterKind, Function, MemoryParam, ParamFindingFailure, Register,
};
use ddbug_parser::FileHash;

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
                            return_type: None,
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
            ty: ddbug_type_to_type(&param.ty(file).map(|x| x.into_owned())),
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
                ty: ddbug_type_to_type(&param.ty(file).map(|x| x.into_owned())),
            });
        }
        return Err(ParamFindingFailure::DwarfNoSize);
    }
    Err(ParamFindingFailure::DwarfNoFrameLocNoReg)
}

fn ddbug_type_to_type(ty: &Option<ddbug_parser::Type>) -> Option<TypeKind> {
    if let Some(ty) = ty {
        return match ty.kind() {
            ddbug_parser::TypeKind::Void => Some(TypeKind::Void),
            ddbug_parser::TypeKind::Base(b) => Some(TypeKind::BaseType(BaseType {
                size: b.byte_size().unwrap(),
                encoding: super::BaseTypeEncoding::Unsigned,
            })),
            _ => None,
        };
    }
    None
}

use std::{borrow, collections::HashMap};

use super::{BaseType, TypeKind};

// TODO: This entire function is a big hack
// beef up this func and remove ddbug dep
// research on how to actually get to the breakpoint line info from a function DIE
pub fn dwarf_get_line_breakpoints(obj: &object::File) -> crate::defs::Result<HashMap<u64, u64>> {
    let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, gimli::Error> {
        match obj.section_by_name(id.name()) {
            Some(ref section) => Ok(section
                .uncompressed_data()
                .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
            None => Ok(borrow::Cow::Borrowed(&[][..])),
        }
    };

    // Load all of the sections.
    let dwarf_cow = gimli::Dwarf::load(&load_section)?;
    let borrow_section: &dyn for<'a> Fn(
        &'a borrow::Cow<[u8]>,
    ) -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, gimli::RunTimeEndian::Little);

    // Create `EndianSlice`s for all of the sections.
    let dwarf = dwarf_cow.borrow(&borrow_section);

    let mut iter = dwarf.units();
    let mut bp_map = HashMap::new();
    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;
        let mut breakpoint_addrs = vec![];
        if let Some(line_program) = unit.line_program.clone() {
            let mut rows = line_program.rows();
            while let Some((_, row)) = rows.next_row()? {
                if row.is_stmt() {
                    breakpoint_addrs.push(row.address());
                }
            }
        }
        breakpoint_addrs.sort();
        debug!(?breakpoint_addrs);

        let mut entries = unit.entries();
        while let Some((_, entry)) = entries.next_dfs()? {
            if entry.tag() == gimli::DW_TAG_subprogram {
                match (
                    entry.attr_value(DW_AT_low_pc)?,
                    entry.attr_value(DW_AT_high_pc)?,
                ) {
                    (Some(AttributeValue::Addr(low_pc)), Some(AttributeValue::Udata(high_pc))) => {
                        debug!(?low_pc, ?high_pc);
                        if let Some(bp) = breakpoint_addrs
                            .iter()
                            .find(|&&x| x > low_pc && x < low_pc + high_pc)
                        {
                            bp_map.insert(low_pc, *bp);
                        }
                    }
                    (_, _) => {}
                }
            }
        }
    }
    Ok(bp_map)
}

fn dwarf_parse_function(
    dwarf: &gimli::Dwarf<gimli::EndianSlice<gimli::RunTimeEndian>>,
    unit: &gimli::Unit<gimli::EndianSlice<gimli::RunTimeEndian>>,
    offset: gimli::UnitOffset,
) -> crate::defs::Result<()> {
    let mut function_tree = unit.entries_tree(Some(offset))?;
    // process_tree(function_tree);
    let root = function_tree.root()?;
    let mut children = root.children();
    while let Some(child) = children.next()? {
        let entry = child.entry();
        let tag = entry.tag();
        println!("{:?}", &tag);
    }
    Ok(())
}
