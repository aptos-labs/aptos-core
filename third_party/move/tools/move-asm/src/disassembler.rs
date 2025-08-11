// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Disassembler for Move bytecode.

use crate::value::AsmValue;
use anyhow::bail;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        Bytecode, CodeOffset, CompiledScript, FieldHandleIndex, LocalIndex, SignatureIndex,
        SignatureToken, VariantFieldHandleIndex, VariantIndex, Visibility,
    },
    module_script_conversion::script_into_module,
    views::{
        FieldDefinitionView, FunctionDefinitionView, FunctionHandleView, ModuleView,
        StructDefinitionView, StructHandleView, ViewInternals,
    },
    CompiledModule,
};
use move_core_types::{
    ability::AbilitySet,
    identifier::Identifier,
    language_storage::{pseudo_script_module_id, ModuleId},
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub fn disassemble_module<T: fmt::Write>(out: T, module: &CompiledModule) -> anyhow::Result<T> {
    Disassembler::run(out, module)
}

pub fn disassemble_script<T: fmt::Write>(out: T, script: &CompiledScript) -> anyhow::Result<T> {
    let script_as_module = script_into_module(script.clone(), "main");
    Disassembler::run(out, &script_as_module)
}

struct Disassembler<T>
where
    T: fmt::Write,
{
    out: T,
    self_module: ModuleId,
    reverse_module_aliases: BTreeMap<ModuleId, Identifier>,
}

impl<T: fmt::Write> Disassembler<T> {
    fn run(out: T, module: &CompiledModule) -> anyhow::Result<T> {
        let version = module.version;
        let module = ModuleView::new(module);
        let mut dis = Disassembler {
            out,
            self_module: module.id(),
            reverse_module_aliases: BTreeMap::new(),
        };
        writeln!(dis.out, "// Bytecode version v{}", version)?;
        if module.id().address == pseudo_script_module_id().address {
            writeln!(dis.out, "script")?;
        } else {
            writeln!(dis.out, "module {}", module.id().short_str_lossless())?;
        }

        // Process used modules, deriving short names for them.
        let mut used_short_names = BTreeSet::new();
        for used_module in module.module_handles() {
            let id = used_module.module_id();
            if id == module.id() {
                continue;
            }
            let mut short_name = id.name.clone();
            while !used_short_names.insert(short_name.clone()) {
                // Make short name unique if needed
                short_name = Identifier::new_unchecked(format!("{}_", short_name))
            }
            writeln!(
                dis.out,
                "use {}{}",
                id.short_str_lossless(),
                if short_name == id.name {
                    "".to_string()
                } else {
                    format!(" as {}", short_name)
                }
            )?;
            dis.reverse_module_aliases.insert(id, short_name);
        }

        // Process friend declarations
        for friend in module.module().friend_decls.iter() {
            let addr = module.module().address_identifier_at(friend.address);
            let name = module.module().identifier_at(friend.name);
            writeln!(dis.out, "friend {}::{}", addr.short_str_lossless(), name)?
        }

        // Process struct and function definitions
        for str in module.structs() {
            dis.struct_(str)?;
            writeln!(dis.out)?
        }
        for (idx, fdef) in module.functions().enumerate() {
            writeln!(dis.out, "// Function definition at index {}", idx)?;
            dis.fun(fdef)?;
            writeln!(dis.out)?
        }

        Ok(dis.out)
    }

    // --------------------------------------------------------------------------------------
    // Structs and Types

    fn struct_(&mut self, str: StructDefinitionView<CompiledModule>) -> anyhow::Result<()> {
        if str.variant_count() == 0 {
            write!(self.out, "struct {}", str.name())?;
        } else {
            write!(self.out, "enum {}", str.name())?;
        }
        if !str.type_parameters().is_empty() {
            self.list(
                str.type_parameters().as_slice(),
                |dis, idx, tparam| dis.type_param_decl(tparam.is_phantom, idx, tparam.constraints),
                "<",
                ", ",
                ">",
            )?
        }
        writeln!(self.out)?;
        if str.variant_count() == 0 {
            self.fields("  ", str.fields_optional_variant(None))?
        } else {
            for variant in 0..str.variant_count() {
                let variant_idx = variant as VariantIndex;
                writeln!(self.out, "  {}", str.variant_name(variant_idx))?;
                self.fields("    ", str.fields_optional_variant(Some(variant_idx)))?
            }
        }
        Ok(())
    }

    fn fields<'a>(
        &mut self,
        indent: &str,
        fields: impl DoubleEndedIterator<Item = FieldDefinitionView<'a, CompiledModule>>,
    ) -> anyhow::Result<()> {
        for field in fields {
            write!(self.out, "{}{}: ", indent, field.name())?;
            self.type_(field.module(), field.signature_token())?;
            writeln!(self.out)?
        }
        Ok(())
    }

    fn type_param_decl(
        &mut self,
        is_phantom: bool,
        idx: usize,
        constraints: AbilitySet,
    ) -> anyhow::Result<()> {
        if is_phantom {
            self.out.write_str("phantom ")?;
        }
        self.out.write_str(&type_param_name(idx))?;
        if !constraints.is_empty() {
            write!(self.out, ": {}", constraints)?
        }
        Ok(())
    }

    fn type_(&mut self, module: &CompiledModule, ty: &SignatureToken) -> anyhow::Result<()> {
        use SignatureToken::*;
        match ty {
            Bool => self.out.write_str("bool")?,
            U8 => self.out.write_str("u8")?,
            U16 => self.out.write_str("u16")?,
            U32 => self.out.write_str("u32")?,
            U64 => self.out.write_str("u64")?,
            U128 => self.out.write_str("u128")?,
            U256 => self.out.write_str("u256")?,
            Address => self.out.write_str("address")?,
            Signer => self.out.write_str("signer")?,
            Vector(elem_ty) => {
                self.out.write_str("vector<")?;
                self.type_(module, elem_ty)?;
                self.out.write_str(">")?;
            },
            Function(params, result, abilities) => {
                self.list(
                    params,
                    |dis, _, e| dis.type_in_function_type(module, e),
                    "|",
                    ", ",
                    "|",
                )?;
                match result.len().cmp(&1) {
                    Ordering::Less => {},
                    Ordering::Equal => self.type_(module, &result[0])?,
                    Ordering::Greater => {
                        self.list(result, |dis, _, e| dis.type_(module, e), "(", ", ", ")")?;
                    },
                }
                self.out
                    .write_str(&abilities.display_postfix().to_string())?;
            },
            Struct(idx) | StructInstantiation(idx, _) => {
                let view = StructHandleView::new(module, module.struct_handle_at(*idx));
                write!(
                    self.out,
                    "{}{}",
                    self.module_id_prefix(&view.module_id())?,
                    view.name()
                )?;
                if let StructInstantiation(_, inst) = ty {
                    self.list(inst, |dis, _, e| dis.type_(module, e), "<", ", ", ">")?
                }
            },
            Reference(elem_ty) => {
                self.out.write_str("&")?;
                self.type_(module, elem_ty)?
            },
            MutableReference(elem_ty) => {
                self.out.write_str("&mut ")?;
                self.type_(module, elem_ty)?
            },
            TypeParameter(idx) => self.out.write_str(&type_param_name(*idx as usize))?,
        }
        Ok(())
    }

    fn type_in_function_type(
        &mut self,
        module: &CompiledModule,
        ty: &SignatureToken,
    ) -> anyhow::Result<()> {
        // The parser cannot deal with `||||`, it must be written as `|(||)|`.
        if matches!(ty, SignatureToken::Function(..)) {
            write!(self.out, "(")?;
            self.type_(module, ty)?;
            write!(self.out, ")")?;
            Ok(())
        } else {
            self.type_(module, ty)
        }
    }

    // --------------------------------------------------------------------------------------
    // Functions

    fn fun(&mut self, fdef: FunctionDefinitionView<CompiledModule>) -> anyhow::Result<()> {
        if !fdef.attributes().is_empty() {
            self.list(
                fdef.attributes(),
                |dis, _, attr| {
                    write!(dis.out, "{}", attr)?;
                    Ok(())
                },
                "#[",
                ", ",
                "]",
            )?;
            write!(self.out, " ")?
        }
        // Function header
        if fdef.is_entry() {
            self.out.write_str("entry ")?
        }
        match fdef.visibility() {
            Visibility::Private => {},
            Visibility::Public => self.out.write_str("public ")?,
            Visibility::Friend => self.out.write_str("friend ")?,
        }
        write!(self.out, "fun {}", fdef.name())?;
        if !fdef.type_parameters().is_empty() {
            self.list(
                fdef.type_parameters().as_slice(),
                |dis, idx, abilities| dis.type_param_decl(false, idx, *abilities),
                "<",
                ", ",
                ">",
            )?
        }
        let arg_tokens = fdef.arg_tokens().collect::<Vec<_>>();
        self.list(
            &arg_tokens,
            |dis, pos, ty| {
                write!(dis.out, "{}: ", local_name(pos as LocalIndex))?;
                dis.type_(fdef.module(), ty.signature_token())
            },
            "(",
            ", ",
            ")",
        )?;
        if fdef.return_count() > 0 {
            self.out.write_str(": ")?;
            if fdef.return_count() > 1 {
                self.list(
                    &fdef.return_tokens().collect::<Vec<_>>(),
                    |dis, _, ty| dis.type_(fdef.module(), ty.signature_token()),
                    "(",
                    ", ",
                    ")",
                )?
            } else {
                self.type_(
                    fdef.module(),
                    fdef.return_tokens().next().unwrap().signature_token(),
                )?
            }
        }
        writeln!(self.out)?;

        if let Some(unit) = fdef.code() {
            // Declare locals
            let locals_sign = fdef.module().signature_at(unit.locals);
            for (pos, ty) in locals_sign.0.iter().enumerate() {
                // The actual local number is # of parameters
                // plus the position in this list.
                write!(
                    self.out,
                    "    local {}: ",
                    local_name((arg_tokens.len() + pos) as LocalIndex)
                )?;
                self.type_(fdef.module(), ty)?;
                writeln!(self.out)?
            }
            // Compute branch labels
            let mut label_map: BTreeMap<CodeOffset, String> = BTreeMap::new();
            for bc in &unit.code {
                match bc {
                    Bytecode::Branch(offs) | Bytecode::BrTrue(offs) | Bytecode::BrFalse(offs) => {
                        let curr_count = label_map.len();
                        label_map
                            .entry(*offs)
                            .or_insert_with(|| format!("l{}", curr_count));
                    },
                    _ => {},
                }
            }
            // Emit code
            for (offs, bc) in unit.code.iter().enumerate() {
                if offs != 0 && offs % 5 == 0 {
                    writeln!(self.out, "    // @{}", offs)?
                }
                if let Some(label) = label_map.get(&(offs as CodeOffset)) {
                    write!(self.out, "{:>2}: ", label)?;
                } else {
                    write!(self.out, "    ")?;
                }
                self.bytecode(fdef.module(), &label_map, bc)?;
                writeln!(self.out)?
            }
        }
        Ok(())
    }

    fn bytecode(
        &mut self,
        module: &CompiledModule,
        label_map: &BTreeMap<CodeOffset, String>,
        bc: &Bytecode,
    ) -> anyhow::Result<()> {
        use Bytecode::*;
        match bc {
            Pop => write!(self.out, "pop")?,
            Ret => write!(self.out, "ret")?,
            BrTrue(offs) | BrFalse(offs) | Branch(offs) => {
                let Some(label) = label_map.get(offs) else {
                    bail!("unexpected code offset without label")
                };
                match bc {
                    Branch(_) => write!(self.out, "branch {}", label)?,
                    BrTrue(_) => write!(self.out, "br_true {}", label)?,
                    BrFalse(_) => write!(self.out, "br_false {}", label)?,
                    _ => unreachable!(),
                }
            },
            LdU8(v) => write!(self.out, "ld_u8 {}", v)?,
            LdU16(v) => write!(self.out, "ld_u16 {}", v)?,
            LdU32(v) => write!(self.out, "ld_u32 {}", v)?,
            LdU64(v) => write!(self.out, "ld_u64 {}", v)?,
            LdU128(v) => write!(self.out, "ld_u128 {}", v)?,
            LdU256(v) => write!(self.out, "ld_u256 {}", v)?,
            CastU8 => write!(self.out, "cast_u8")?,
            CastU16 => write!(self.out, "cast_u16")?,
            CastU32 => write!(self.out, "cast_u32")?,
            CastU64 => write!(self.out, "cast_u64")?,
            CastU128 => write!(self.out, "cast_u128")?,
            CastU256 => write!(self.out, "cast_u256")?,
            LdConst(hdl) => {
                write!(self.out, "ld_const")?;
                let cons = module.constant_at(*hdl);
                write!(self.out, "<")?;
                self.type_(module, &cons.type_)?;
                write!(self.out, ">")?;
                if let Some(val) = cons
                    .deserialize_constant()
                    .and_then(|v| AsmValue::from_move_value(&v).ok())
                {
                    write!(self.out, " {}", val)?
                } else {
                    write!(self.out, " <invalid constant>")?
                }
            },
            LdTrue => write!(self.out, "ld_true")?,
            LdFalse => write!(self.out, "ld_false")?,
            CopyLoc(loc) => write!(self.out, "copy_loc {}", local_name(*loc))?,
            MoveLoc(loc) => write!(self.out, "move_loc {}", local_name(*loc))?,
            StLoc(loc) => write!(self.out, "st_loc {}", local_name(*loc))?,
            Call(idx) => {
                let view = FunctionHandleView::new(module, module.function_handle_at(*idx));
                write!(
                    self.out,
                    "call {}{}",
                    self.module_id_prefix(&view.module_id())?,
                    view.name()
                )?
            },
            CallGeneric(idx) => {
                let inst_handle = module.function_instantiation_at(*idx);
                let view =
                    FunctionHandleView::new(module, module.function_handle_at(inst_handle.handle));
                write!(
                    self.out,
                    "call {}{}",
                    self.module_id_prefix(&view.module_id())?,
                    view.name()
                )?;
                self.ty_args(module, inst_handle.type_parameters)?
            },
            Pack(idx) | Unpack(idx) => {
                let op = if matches!(bc, Pack(_)) {
                    "pack"
                } else {
                    "unpack"
                };
                let view = StructDefinitionView::new(module, module.struct_def_at(*idx));
                write!(self.out, "{} {}", op, view.name())?
            },
            PackGeneric(idx) | UnpackGeneric(idx) => {
                let op = if matches!(bc, PackGeneric(_)) {
                    "pack"
                } else {
                    "unpack"
                };
                let inst_handle = module.struct_instantiation_at(*idx);
                let view = StructDefinitionView::new(module, module.struct_def_at(inst_handle.def));
                write!(self.out, "{} {}", op, view.name())?;
                self.ty_args(module, inst_handle.type_parameters)?
            },
            PackVariant(idx) | UnpackVariant(idx) | TestVariant(idx) => {
                let op = match bc {
                    PackVariant(_) => "pack_variant",
                    UnpackVariant(_) => "unpack_variant",
                    _ => "test_variant",
                };
                let handle = module.struct_variant_handle_at(*idx);
                let view =
                    StructDefinitionView::new(module, module.struct_def_at(handle.struct_index));
                let variant_name = view.variant_name(handle.variant);
                write!(self.out, "{} {}, {}", op, view.name(), variant_name)?
            },
            PackVariantGeneric(idx) | UnpackVariantGeneric(idx) | TestVariantGeneric(idx) => {
                let op = match bc {
                    PackVariantGeneric(_) => "pack_variant",
                    UnpackVariantGeneric(_) => "unpack_variant",
                    _ => "test_variant",
                };
                let inst_handle = module.struct_variant_instantiation_at(*idx);
                let handle = module.struct_variant_handle_at(inst_handle.handle);
                let view =
                    StructDefinitionView::new(module, module.struct_def_at(handle.struct_index));
                let variant_name = view.variant_name(handle.variant);
                write!(self.out, "{} {}", op, view.name())?;
                self.ty_args(module, inst_handle.type_parameters)?;
                write!(self.out, ", {}", variant_name)?
            },
            ReadRef => write!(self.out, "read_ref")?,
            WriteRef => write!(self.out, "write_ref")?,
            FreezeRef => write!(self.out, "freeze_ref")?,
            MutBorrowLoc(idx) => write!(self.out, "mut_borrow_loc {}", local_name(*idx))?,
            ImmBorrowLoc(idx) => write!(self.out, "borrow_loc {}", local_name(*idx))?,
            MutBorrowField(idx) => self.borrow_field(module, true, *idx, None)?,
            ImmBorrowField(idx) => self.borrow_field(module, false, *idx, None)?,
            MutBorrowFieldGeneric(idx) => {
                let inst_handle = module.field_instantiation_at(*idx);
                self.borrow_field(
                    module,
                    true,
                    inst_handle.handle,
                    Some(inst_handle.type_parameters),
                )?
            },
            ImmBorrowFieldGeneric(idx) => {
                let inst_handle = module.field_instantiation_at(*idx);
                self.borrow_field(
                    module,
                    false,
                    inst_handle.handle,
                    Some(inst_handle.type_parameters),
                )?
            },
            MutBorrowVariantField(idx) => self.borrow_variant_field(module, true, *idx, None)?,
            ImmBorrowVariantField(idx) => self.borrow_variant_field(module, false, *idx, None)?,
            MutBorrowVariantFieldGeneric(idx) => {
                let inst_handle = module.variant_field_instantiation_at(*idx);
                self.borrow_variant_field(
                    module,
                    true,
                    inst_handle.handle,
                    Some(inst_handle.type_parameters),
                )?
            },
            ImmBorrowVariantFieldGeneric(idx) => {
                let inst_handle = module.variant_field_instantiation_at(*idx);
                self.borrow_variant_field(
                    module,
                    false,
                    inst_handle.handle,
                    Some(inst_handle.type_parameters),
                )?
            },
            MutBorrowGlobal(idx) | ImmBorrowGlobal(idx) | Exists(idx) | MoveFrom(idx)
            | MoveTo(idx) => {
                let op = match bc {
                    MutBorrowGlobal(_) => "mut_borrow_global",
                    ImmBorrowGlobal(_) => "borrow_global",
                    Exists(_) => "exists",
                    MoveFrom(_) => "move_from",
                    MoveTo(_) => "move_to",
                    _ => unreachable!(),
                };
                let view = StructDefinitionView::new(module, module.struct_def_at(*idx));
                write!(self.out, "{} {}", op, view.name())?
            },
            MutBorrowGlobalGeneric(idx)
            | ImmBorrowGlobalGeneric(idx)
            | ExistsGeneric(idx)
            | MoveFromGeneric(idx)
            | MoveToGeneric(idx) => {
                let op = match bc {
                    MutBorrowGlobalGeneric(_) => "mut_borrow_global",
                    ImmBorrowGlobalGeneric(_) => "borrow_global",
                    ExistsGeneric(_) => "exists",
                    MoveFromGeneric(_) => "move_from",
                    MoveToGeneric(_) => "move_to",
                    _ => unreachable!(),
                };
                let inst_handle = module.struct_instantiation_at(*idx);
                let view = StructDefinitionView::new(module, module.struct_def_at(inst_handle.def));
                write!(self.out, "{} {}", op, view.name())?;
                self.ty_args(module, inst_handle.type_parameters)?
            },
            Add => write!(self.out, "add")?,
            Sub => write!(self.out, "sub")?,
            Mul => write!(self.out, "mul")?,
            Mod => write!(self.out, "mod")?,
            Div => write!(self.out, "div")?,
            BitOr => write!(self.out, "bit_or")?,
            BitAnd => write!(self.out, "bit_and")?,
            Xor => write!(self.out, "xor")?,
            Or => write!(self.out, "or")?,
            And => write!(self.out, "and")?,
            Not => write!(self.out, "not")?,
            Eq => write!(self.out, "eq")?,
            Neq => write!(self.out, "neq")?,
            Lt => write!(self.out, "lt")?,
            Gt => write!(self.out, "gt")?,
            Le => write!(self.out, "le")?,
            Ge => write!(self.out, "ge")?,
            Abort => write!(self.out, "abort")?,
            Nop => write!(self.out, "nop")?,
            Shl => write!(self.out, "shl")?,
            Shr => write!(self.out, "shr")?,
            VecPack(idx, len) => {
                write!(self.out, "vec_pack ")?;
                self.ty_args(module, *idx)?;
                write!(self.out, ", {}", len)?;
            },
            VecLen(idx) => {
                write!(self.out, "vec_len ")?;
                self.ty_args(module, *idx)?;
            },
            VecImmBorrow(idx) => {
                write!(self.out, "vec_borrow ")?;
                self.ty_args(module, *idx)?;
            },
            VecMutBorrow(idx) => {
                write!(self.out, "vec_mut_borrow ")?;
                self.ty_args(module, *idx)?;
            },
            VecPushBack(idx) => {
                write!(self.out, "vec_push_back ")?;
                self.ty_args(module, *idx)?;
            },
            VecPopBack(idx) => {
                write!(self.out, "vec_pop_back ")?;
                self.ty_args(module, *idx)?;
            },
            VecUnpack(idx, len) => {
                write!(self.out, "vec_unpack ")?;
                self.ty_args(module, *idx)?;
                write!(self.out, ", {}", len)?;
            },
            VecSwap(idx) => {
                write!(self.out, "vec_swap ")?;
                self.ty_args(module, *idx)?;
            },
            PackClosure(idx, mask) => {
                let view = FunctionHandleView::new(module, module.function_handle_at(*idx));
                write!(
                    self.out,
                    "pack_closure {}{}, {}",
                    self.module_id_prefix(&view.module_id())?,
                    view.name(),
                    mask
                )?
            },
            PackClosureGeneric(idx, mask) => {
                let inst_handle = module.function_instantiation_at(*idx);
                let view =
                    FunctionHandleView::new(module, module.function_handle_at(inst_handle.handle));
                write!(
                    self.out,
                    "pack_closure {}{}",
                    self.module_id_prefix(&view.module_id())?,
                    view.name()
                )?;
                self.ty_args(module, inst_handle.type_parameters)?;
                write!(self.out, ", {}", mask)?
            },
            CallClosure(idx) => {
                write!(self.out, "call_closure ")?;
                self.ty_args(module, *idx)?;
            },
        }
        Ok(())
    }

    fn borrow_field(
        &mut self,
        module: &CompiledModule,
        is_mut: bool,
        field_idx: FieldHandleIndex,
        inst_opt: Option<SignatureIndex>,
    ) -> anyhow::Result<()> {
        let op = if is_mut {
            "mut_borrow_field"
        } else {
            "borrow_field"
        };
        let handle = module.field_handle_at(field_idx);
        let view = StructDefinitionView::new(module, module.struct_def_at(handle.owner));
        let field_name = view
            .fields_optional_variant(None)
            .nth(handle.field as usize)
            .map_or("<index-error>".to_string(), |f| f.name().to_string());
        write!(self.out, "{} {}", op, view.name())?;
        if let Some(inst) = inst_opt {
            self.ty_args(module, inst)?
        }
        write!(self.out, ", {}", field_name)?;
        Ok(())
    }

    fn borrow_variant_field(
        &mut self,
        module: &CompiledModule,
        is_mut: bool,
        field_idx: VariantFieldHandleIndex,
        inst_opt: Option<SignatureIndex>,
    ) -> anyhow::Result<()> {
        let op = if is_mut {
            "mut_borrow_variant_field"
        } else {
            "borrow_variant_field"
        };
        let handle = module.variant_field_handle_at(field_idx);
        let view = StructDefinitionView::new(module, module.struct_def_at(handle.struct_index));
        write!(self.out, "{} {}", op, view.name())?;
        if let Some(inst) = inst_opt {
            self.ty_args(module, inst)?
        }
        for variant_idx in &handle.variants {
            let field_name = view
                .fields_optional_variant(Some(*variant_idx))
                .nth(handle.field as usize)
                .map_or("<index-bound-error>".to_string(), |f| f.name().to_string());
            write!(
                self.out,
                ", {}::{}",
                view.variant_name(*variant_idx),
                field_name
            )?
        }
        Ok(())
    }

    fn ty_args(&mut self, module: &CompiledModule, sign_idx: SignatureIndex) -> anyhow::Result<()> {
        let sign = module.signature_at(sign_idx);
        self.list(&sign.0, |dis, _, e| dis.type_(module, e), "<", ", ", ">")
    }

    // --------------------------------------------------------------------------------------
    // General Helpers

    fn module_id_prefix(&self, module_id: &ModuleId) -> anyhow::Result<String> {
        if &self.self_module == module_id {
            Ok(format!("{}::", self.self_module.name))
        } else if let Some(short) = self.reverse_module_aliases.get(module_id) {
            Ok(format!("{}::", short))
        } else {
            Ok(format!("{}::", module_id.short_str_lossless()))
        }
    }

    fn list<E>(
        &mut self,
        elems: &[E],
        writer: impl Fn(&mut Self, usize, &E) -> anyhow::Result<()>,
        open: &str,
        sep: &str,
        close: &str,
    ) -> anyhow::Result<()> {
        self.out.write_str(open)?;
        for (count, elem) in elems.iter().enumerate() {
            if count > 0 {
                self.out.write_str(sep)?
            }
            writer(self, count, elem)?;
        }
        self.out.write_str(close)?;
        Ok(())
    }
}

fn type_param_name(idx: usize) -> String {
    format!("T{}", idx)
}

fn local_name(idx: LocalIndex) -> String {
    format!("l{}", idx)
}
