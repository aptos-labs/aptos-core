// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type checking of XIR function bodies.
//!
//! Every instruction must be well typed under Move's rules before it is
//! translated. Translation keeps only part of what a document says — local
//! types survive, but an arithmetic width or a constant's kind does not — so
//! a mismatch that is not rejected here is either caught late by the bytecode
//! verifier, with no reference to the document, or not caught at all.
//!
//! The rules follow the operand layouts the translator already relies on,
//! including where it accepts either a value or a reference.

use super::*;

impl FunctionTranslator<'_> {
    /// Checks every instruction and terminator of the function.
    pub(super) fn check_types(&self) -> Result<()> {
        for (block_id, block) in self.decl.blocks.iter().enumerate() {
            for (instr_id, instr) in block.instrs.iter().enumerate() {
                self.check_instruction(instr).with_context(|| {
                    format!(
                        "function `{}`, block {block_id}, instruction {instr_id}",
                        self.decl.name
                    )
                })?;
            }
            self.check_term(&block.term)
                .with_context(|| format!("function `{}`, block {block_id}", self.decl.name))?;
        }
        Ok(())
    }

    fn check_instruction(&self, instr: &Instr) -> Result<()> {
        match instr {
            Instr::Load(dst, constant) => {
                let ty = self.local(*dst)?;
                let fits = match constant {
                    Constant::Num(_) => ty.is_number(),
                    Constant::Bool(_) => *ty == Type::Primitive(PrimitiveType::Bool),
                    Constant::Address(_) => *ty == Type::Primitive(PrimitiveType::Address),
                    // Refused when the constant is translated.
                    Constant::Vector(_) => true,
                };
                ensure!(
                    fits,
                    "load: constant {constant:?} does not fit l{dst} of type `{}`",
                    self.show(ty)
                );
                Ok(())
            },
            Instr::Assign(dst, src) => {
                let expected = self.local(*src)?.clone();
                self.expect("assign", "destination", *dst, &expected)
            },
            Instr::Call(dsts, oper, srcs) => self.check_call(dsts, oper, srcs),
            Instr::Nop => Ok(()),
        }
    }

    fn check_term(&self, term: &Term) -> Result<()> {
        match term {
            Term::Jump(_) => Ok(()),
            Term::Branch(cond, _, _) => self.expect("branch", "condition", *cond, &bool_type()),
            Term::Abort(code) => self.expect("abort", "code", *code, &u64_type()),
            Term::Ret(values) => {
                ensure!(
                    values.len() == self.decl.returns.len(),
                    "return arity mismatch"
                );
                let declared = self.type_args(&self.decl.returns)?;
                for (position, (value, ty)) in values.iter().zip(&declared).enumerate() {
                    self.expect("ret", format_args!("value {position}"), *value, ty)?;
                }
                Ok(())
            },
        }
    }

    /// Checks one operation: the types its operands and results must have,
    /// given the operation and, where the rule depends on them, the operands.
    fn check_call(&self, dsts: &[usize], oper: &Oper, srcs: &[usize]) -> Result<()> {
        for id in dsts.iter().chain(srcs) {
            self.local(*id)?;
        }
        let (expected_srcs, expected_dsts) = match self.signature(dsts, oper, srcs) {
            Ok(signature) => signature,
            // Too few operands to work out the rule: leave it to the
            // translator's arity check, which names the expected counts.
            Err(error) if error.is::<MissingOperand>() => return Ok(()),
            Err(error) => return Err(error),
        };
        arity(dsts, srcs, expected_dsts.len(), expected_srcs.len(), oper)?;
        for (position, (src, ty)) in srcs.iter().zip(&expected_srcs).enumerate() {
            self.expect(
                format_args!("{oper:?}"),
                format_args!("operand {position}"),
                *src,
                ty,
            )?;
        }
        for (position, (dst, ty)) in dsts.iter().zip(&expected_dsts).enumerate() {
            self.expect(
                format_args!("{oper:?}"),
                format_args!("result {position}"),
                *dst,
                ty,
            )?;
        }
        Ok(())
    }

    /// The operand and result types an operation requires.
    ///
    /// Where the translator accepts a value or a reference, or a shared or
    /// mutable reference, the expected type is taken from the actual operand
    /// once its shape has been checked, so the comparison afterwards passes
    /// for every accepted form and fails for anything else.
    fn signature(
        &self,
        dsts: &[usize],
        oper: &Oper,
        srcs: &[usize],
    ) -> Result<(Vec<Type>, Vec<Type>)> {
        let src = |i: usize| -> Result<Type> {
            let id = srcs.get(i).ok_or(MissingOperand)?;
            Ok(self.local(*id)?.clone())
        };
        let dst = |i: usize| -> Result<Type> {
            let id = dsts.get(i).ok_or(MissingOperand)?;
            Ok(self.local(*id)?.clone())
        };
        Ok(match oper {
            Oper::Add(width)
            | Oper::Sub(width)
            | Oper::Mul(width)
            | Oper::Div(width)
            | Oper::Mod(width)
            | Oper::BitAnd(width)
            | Oper::BitOr(width)
            | Oper::BitXor(width) => {
                let ty = int_type(*width);
                (vec![ty.clone(), ty.clone()], vec![ty])
            },
            Oper::Shl(width) | Oper::Shr(width) => {
                let ty = int_type(*width);
                (vec![ty.clone(), int_type(IntType::U8)], vec![ty])
            },
            Oper::Cast(target) => {
                let operand = self.integer(src(0)?, oper)?;
                (vec![operand], vec![int_type(*target)])
            },
            Oper::Lt | Oper::Eq => {
                let operand = src(0)?;
                (vec![operand.clone(), operand], vec![bool_type()])
            },
            // Only `lt` is lowered through `std::cmp`, so `le` needs integers.
            Oper::Le => {
                let operand = self.integer(src(0)?, oper)?;
                (vec![operand.clone(), operand], vec![bool_type()])
            },
            Oper::And | Oper::Or => (vec![bool_type(), bool_type()], vec![bool_type()]),
            Oper::Not => (vec![bool_type()], vec![bool_type()]),
            Oper::Pack => self.pack_rule(dst(0)?, &[], None, oper)?,
            Oper::PackInst(args) => self.pack_rule(dst(0)?, args, None, oper)?,
            Oper::PackVariant(variant) => self.pack_rule(dst(0)?, &[], Some(*variant), oper)?,
            Oper::PackVariantInst(variant, args) => {
                self.pack_rule(dst(0)?, args, Some(*variant), oper)?
            },
            Oper::Unpack => self.unpack_rule(src(0)?, &[], None, oper)?,
            Oper::UnpackInst(args) => self.unpack_rule(src(0)?, args, None, oper)?,
            Oper::UnpackVariant(variant) => self.unpack_rule(src(0)?, &[], Some(*variant), oper)?,
            Oper::UnpackVariantInst(variant, args) => {
                self.unpack_rule(src(0)?, args, Some(*variant), oper)?
            },
            Oper::TestVariant(variant) => {
                self.test_variant_rule(src(0)?, &[], *variant, false, oper)?
            },
            Oper::TestVariantInst(variant, args) => {
                self.test_variant_rule(src(0)?, args, *variant, false, oper)?
            },
            Oper::TestVariantRef(variant) => {
                self.test_variant_rule(src(0)?, &[], *variant, true, oper)?
            },
            Oper::TestVariantRefInst(variant, args) => {
                self.test_variant_rule(src(0)?, args, *variant, true, oper)?
            },
            Oper::GetField(field) => self.get_field_rule(src(0)?, &[], *field, oper)?,
            Oper::GetFieldInst(field, args) => self.get_field_rule(src(0)?, args, *field, oper)?,
            Oper::BorrowField(field) => {
                self.borrow_field_rule(src(0)?, dst(0)?, &[], None, *field, oper)?
            },
            Oper::BorrowFieldInst(field, args) => {
                self.borrow_field_rule(src(0)?, dst(0)?, args, None, *field, oper)?
            },
            Oper::BorrowVariantField(variants, field) => {
                self.borrow_field_rule(src(0)?, dst(0)?, &[], Some(variants), *field, oper)?
            },
            Oper::BorrowVariantFieldInst(variants, field, args) => {
                self.borrow_field_rule(src(0)?, dst(0)?, args, Some(variants), *field, oper)?
            },
            Oper::UpdateField(_) => bail!("unsupported XIR operation {oper:?}"),
            Oper::VecPack => {
                let target = dst(0)?;
                let element = self.vector_element(&target, oper)?;
                (vec![element; srcs.len()], vec![target])
            },
            Oper::VecLen => {
                let source = src(0)?;
                self.vector_element_behind_reference(&source, oper)?;
                (vec![source], vec![u64_type()])
            },
            Oper::VecGet => {
                let source = src(0)?;
                let element = self.vector_element_behind_reference(&source, oper)?;
                (vec![source, u64_type()], vec![element])
            },
            Oper::VecSet | Oper::VecInsert => {
                let vector = src(0)?;
                let element = self.vector_element(&vector, oper)?;
                (vec![vector.clone(), u64_type(), element], vec![vector])
            },
            Oper::VecPush => {
                let vector = src(0)?;
                let element = self.vector_element(&vector, oper)?;
                (vec![vector.clone(), element], vec![vector])
            },
            Oper::VecPop => {
                let vector = src(0)?;
                let element = self.vector_element(&vector, oper)?;
                (vec![vector.clone()], vec![vector, element])
            },
            Oper::VecRemove => {
                let vector = src(0)?;
                let element = self.vector_element(&vector, oper)?;
                (vec![vector.clone(), u64_type()], vec![vector, element])
            },
            Oper::VecSwap => {
                let vector = src(0)?;
                self.vector_element(&vector, oper)?;
                (vec![vector.clone(), u64_type(), u64_type()], vec![vector])
            },
            Oper::BorrowVecElem => {
                let source = src(0)?;
                let (kind, referent) = self.reference(&source, oper, "operand 0")?;
                let element = self.vector_element(&referent, oper)?;
                let result = self.borrow_result(dst(0)?, kind, element, oper)?;
                (vec![source, u64_type()], vec![result])
            },
            Oper::GetGlobal(id) | Oper::MoveFrom(id) => {
                (vec![address_type()], vec![self.resource(*id, &[], oper)?])
            },
            Oper::GetGlobalInst(id, args) | Oper::MoveFromInst(id, args) => {
                (vec![address_type()], vec![self.resource(*id, args, oper)?])
            },
            Oper::WriteGlobal(id) => {
                let resource = self.resource(*id, &[], oper)?;
                (vec![address_type(), resource], vec![])
            },
            Oper::MoveTo(id) => self.move_to_rule(src(0)?, self.resource(*id, &[], oper)?, oper)?,
            Oper::MoveToInst(id, args) => {
                self.move_to_rule(src(0)?, self.resource(*id, args, oper)?, oper)?
            },
            Oper::Exists(id) => {
                self.resource(*id, &[], oper)?;
                (vec![address_type()], vec![bool_type()])
            },
            Oper::ExistsInst(id, args) => {
                self.resource(*id, args, oper)?;
                (vec![address_type()], vec![bool_type()])
            },
            Oper::BorrowGlobal(id) => {
                let resource = self.resource(*id, &[], oper)?;
                let result = self.borrow_result(dst(0)?, ReferenceKind::Mutable, resource, oper)?;
                (vec![address_type()], vec![result])
            },
            Oper::BorrowGlobalInst(id, args) => {
                let resource = self.resource(*id, args, oper)?;
                let result = self.borrow_result(dst(0)?, ReferenceKind::Mutable, resource, oper)?;
                (vec![address_type()], vec![result])
            },
            Oper::Function(id) => self.call_rule(*id, &[], oper)?,
            Oper::FunctionInst(id, args) => self.call_rule(*id, args, oper)?,
            Oper::BorrowLoc => {
                let source = src(0)?;
                ensure!(
                    !source.is_reference(),
                    "{oper:?}: operand 0 is already a reference, `{}`",
                    self.show(&source)
                );
                let result =
                    self.borrow_result(dst(0)?, ReferenceKind::Mutable, source.clone(), oper)?;
                (vec![source], vec![result])
            },
            Oper::ReadRef => {
                let source = src(0)?;
                let (_, referent) = self.reference(&source, oper, "operand 0")?;
                (vec![source], vec![referent])
            },
            Oper::WriteRef => {
                let target = src(0)?;
                let referent = self.mutable_referent(&target, oper)?;
                (vec![target, referent], vec![])
            },
            Oper::FreezeRef => {
                let source = src(0)?;
                let referent = self.mutable_referent(&source, oper)?;
                (vec![source], vec![Type::Reference(
                    ReferenceKind::Immutable,
                    Box::new(referent),
                )])
            },
        })
    }

    /// Requires local `id` to have type `expected`.
    fn expect(
        &self,
        what: impl std::fmt::Display,
        position: impl std::fmt::Display,
        id: usize,
        expected: &Type,
    ) -> Result<()> {
        let actual = self.local(id)?;
        ensure!(
            actual == expected,
            "{what}: {position} (l{id}) is `{}`, expected `{}`",
            self.show(actual),
            self.show(expected)
        );
        Ok(())
    }

    /// The struct and type arguments of `ty`, which must be a struct of this
    /// module; the translator resolves only those.
    fn struct_instance(&self, ty: &Type, oper: &Oper) -> Result<(StructId, Vec<Type>)> {
        match ty {
            Type::Struct(mid, sid, args) if *mid == self.module_id => Ok((*sid, args.clone())),
            other => bail!(
                "{oper:?}: `{}` is not a struct of this module",
                self.show(other)
            ),
        }
    }

    /// Resolves a struct operand: its declaration, and type arguments that
    /// must equal the operation's own, match the declared parameter count,
    /// and not be references.
    fn struct_operand(
        &self,
        ty: &Type,
        given: &[Ty],
        oper: &Oper,
    ) -> Result<(StructId, Vec<Type>)> {
        let (sid, args) = self.struct_instance(ty, oper)?;
        let given = self.type_args(given)?;
        ensure!(
            given == args,
            "{oper:?}: type arguments do not match the operand's `{}`",
            self.show(ty)
        );
        self.check_type_args(sid, &args, oper)?;
        Ok((sid, args))
    }

    /// A struct's type arguments must match its declared parameters in
    /// number, or instantiating its fields indexes past them, and must not
    /// be references.
    fn check_type_args(&self, sid: StructId, args: &[Type], oper: &Oper) -> Result<()> {
        let decl = &self.xir.structs[self.struct_index(sid)?];
        ensure!(
            args.len() == decl.type_parameters.len(),
            "{oper:?}: `{}` takes {} type arguments, got {}",
            decl.name,
            decl.type_parameters.len(),
            args.len()
        );
        self.no_reference_args(args, oper)
    }

    fn no_reference_args(&self, args: &[Type], oper: &Oper) -> Result<()> {
        for arg in args {
            ensure!(
                !arg.is_reference(),
                "{oper:?}: type argument `{}` is a reference",
                self.show(arg)
            );
        }
        Ok(())
    }

    fn struct_index(&self, sid: StructId) -> Result<usize> {
        self.struct_ids
            .iter()
            .position(|candidate| *candidate == sid)
            .context("unknown local struct")
    }

    fn pack_rule(
        &self,
        target: Type,
        given: &[Ty],
        variant: Option<usize>,
        oper: &Oper,
    ) -> Result<(Vec<Type>, Vec<Type>)> {
        let (sid, args) = self.struct_operand(&target, given, oper)?;
        Ok((self.field_types(sid, &args, variant)?, vec![target]))
    }

    fn unpack_rule(
        &self,
        source: Type,
        given: &[Ty],
        variant: Option<usize>,
        oper: &Oper,
    ) -> Result<(Vec<Type>, Vec<Type>)> {
        let (sid, args) = self.struct_operand(&source, given, oper)?;
        let fields = self.field_types(sid, &args, variant)?;
        Ok((vec![source], fields))
    }

    /// `test_variant` takes the enum by value, `test_variant_ref` by either
    /// kind of reference.
    fn test_variant_rule(
        &self,
        source: Type,
        given: &[Ty],
        variant: usize,
        by_reference: bool,
        oper: &Oper,
    ) -> Result<(Vec<Type>, Vec<Type>)> {
        let value = if by_reference {
            self.reference(&source, oper, "operand 0")?.1
        } else {
            source.clone()
        };
        let (sid, args) = self.struct_operand(&value, given, oper)?;
        self.field_types(sid, &args, Some(variant))?;
        Ok((vec![source], vec![bool_type()]))
    }

    fn get_field_rule(
        &self,
        source: Type,
        given: &[Ty],
        field: usize,
        oper: &Oper,
    ) -> Result<(Vec<Type>, Vec<Type>)> {
        let (sid, args) = self.struct_operand(&source, given, oper)?;
        let field_type = self.field_type(sid, &args, None, field)?;
        Ok((vec![source], vec![field_type]))
    }

    /// `borrow_field`, or with `variants`, `borrow_variant_field`, whose field
    /// must have one type across every named variant.
    fn borrow_field_rule(
        &self,
        source: Type,
        result: Type,
        given: &[Ty],
        variants: Option<&[usize]>,
        field: usize,
        oper: &Oper,
    ) -> Result<(Vec<Type>, Vec<Type>)> {
        let (kind, referent) = self.reference(&source, oper, "operand 0")?;
        let (sid, args) = self.struct_operand(&referent, given, oper)?;
        let field_type = match variants {
            None => self.field_type(sid, &args, None, field)?,
            Some(variants) => {
                let (first, others) = variants
                    .split_first()
                    .with_context(|| format!("{oper:?} names no variant"))?;
                let field_type = self.field_type(sid, &args, Some(*first), field)?;
                for variant in others {
                    let other = self.field_type(sid, &args, Some(*variant), field)?;
                    ensure!(
                        other == field_type,
                        "{oper:?}: field {field} has different types in the named variants"
                    );
                }
                field_type
            },
        };
        let result = self.borrow_result(result, kind, field_type, oper)?;
        Ok((vec![source], vec![result]))
    }

    /// `move_to` takes a signer or a reference to one, then the resource.
    fn move_to_rule(
        &self,
        signer: Type,
        resource: Type,
        oper: &Oper,
    ) -> Result<(Vec<Type>, Vec<Type>)> {
        ensure!(
            signer.skip_reference().is_signer(),
            "{oper:?}: operand 0 is `{}`, expected a signer or a reference to one",
            self.show(&signer)
        );
        Ok((vec![signer, resource], vec![]))
    }

    /// A call's operands and results are the callee's signature, instantiated.
    fn call_rule(&self, id: usize, given: &[Ty], oper: &Oper) -> Result<(Vec<Type>, Vec<Type>)> {
        let target = function_at(self.env, self.xir, self.module_id, self.function_ids, id)?;
        let callee = self.env.get_function(target);
        let args = self.type_args(given)?;
        ensure!(
            args.len() == callee.get_type_parameter_count(),
            "{oper:?}: `{}` takes {} type arguments, got {}",
            callee.get_full_name_str(),
            callee.get_type_parameter_count(),
            args.len()
        );
        self.no_reference_args(&args, oper)?;
        let params = callee
            .get_parameter_types()
            .iter()
            .map(|ty| ty.instantiate(&args))
            .collect();
        let results = callee
            .get_result_type()
            .flatten()
            .iter()
            .map(|ty| ty.instantiate(&args))
            .collect();
        Ok((params, results))
    }

    /// The field types of struct `sid`, or of one of its variants,
    /// instantiated with `args`.
    fn field_types(
        &self,
        sid: StructId,
        args: &[Type],
        variant: Option<usize>,
    ) -> Result<Vec<Type>> {
        let decl = &self.xir.structs[self.struct_index(sid)?];
        // `instantiate` indexes `args` by parameter number, so a short list
        // would panic rather than fail.
        ensure!(
            args.len() == decl.type_parameters.len(),
            "`{}` takes {} type arguments, got {}",
            decl.name,
            decl.type_parameters.len(),
            args.len()
        );
        let fields = match (variant, &decl.variants) {
            (None, None) => &decl.fields,
            (Some(variant), Some(variants)) => {
                &variants
                    .get(variant)
                    .with_context(|| format!("variant id {variant} is out of range"))?
                    .fields
            },
            (None, Some(_)) => bail!("`{}` is an enum, not a struct", decl.name),
            (Some(_), None) => bail!("`{}` is a struct, not an enum", decl.name),
        };
        let scope = self.scope();
        fields
            .iter()
            .map(|field| Ok(model_type(&field.ty, &scope)?.instantiate(args)))
            .collect()
    }

    fn field_type(
        &self,
        sid: StructId,
        args: &[Type],
        variant: Option<usize>,
        field: usize,
    ) -> Result<Type> {
        self.field_types(sid, args, variant)?
            .into_iter()
            .nth(field)
            .with_context(|| format!("field id {field} is out of range"))
    }

    /// The resource a global-storage operation names, with its type
    /// arguments. Global storage is local-only in Move.
    fn resource(&self, id: usize, given: &[Ty], oper: &Oper) -> Result<Type> {
        let sid = struct_at(self.struct_ids, id, &self.decl.name)?;
        let args = self.type_args(given)?;
        self.check_type_args(sid, &args, oper)?;
        Ok(Type::Struct(self.module_id, sid, args))
    }

    /// Splits a reference type into its kind and referent.
    fn reference(&self, ty: &Type, oper: &Oper, position: &str) -> Result<(ReferenceKind, Type)> {
        match ty {
            Type::Reference(kind, referent) => Ok((*kind, referent.as_ref().clone())),
            other => bail!(
                "{oper:?}: {position} is `{}`, expected a reference",
                self.show(other)
            ),
        }
    }

    /// Requires `ty`, the type of operand 0, to be an integer.
    fn integer(&self, ty: Type, oper: &Oper) -> Result<Type> {
        ensure!(
            ty.is_number(),
            "{oper:?}: operand 0 is `{}`, expected an integer",
            self.show(&ty)
        );
        Ok(ty)
    }

    /// The referent of `ty`, the type of operand 0, which must be a mutable reference.
    fn mutable_referent(&self, ty: &Type, oper: &Oper) -> Result<Type> {
        let (kind, referent) = self.reference(ty, oper, "operand 0")?;
        ensure!(
            kind == ReferenceKind::Mutable,
            "{oper:?}: operand 0 is `{}`, expected a mutable reference",
            self.show(ty)
        );
        Ok(referent)
    }

    /// The expected result of a borrow of `referent` from a source of kind
    /// `source`: the result may be shared, or mutable if the source is.
    fn borrow_result(
        &self,
        result: Type,
        source: ReferenceKind,
        referent: Type,
        oper: &Oper,
    ) -> Result<Type> {
        let kind = match &result {
            Type::Reference(ReferenceKind::Mutable, _) => {
                ensure!(
                    source == ReferenceKind::Mutable,
                    "{oper:?}: a mutable reference cannot be borrowed from a shared one"
                );
                ReferenceKind::Mutable
            },
            _ => ReferenceKind::Immutable,
        };
        Ok(Type::Reference(kind, Box::new(referent)))
    }

    fn vector_element(&self, ty: &Type, oper: &Oper) -> Result<Type> {
        match ty {
            Type::Vector(element) => Ok(element.as_ref().clone()),
            other => bail!("{oper:?}: `{}` is not a vector", self.show(other)),
        }
    }

    /// The element type of a vector, or of a reference to one.
    fn vector_element_behind_reference(&self, ty: &Type, oper: &Oper) -> Result<Type> {
        self.vector_element(ty.skip_reference(), oper)
    }

    pub(super) fn scope(&self) -> StructScope<'_> {
        StructScope {
            module_id: self.module_id,
            local: self.struct_ids,
            external: self.external_struct_ids,
        }
    }

    fn show(&self, ty: &Type) -> String {
        ty.display(&self.env.get_type_display_ctx()).to_string()
    }
}

/// A rule needed an operand or result the instruction does not have.
#[derive(Debug)]
struct MissingOperand;

impl std::fmt::Display for MissingOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing operand")
    }
}

impl std::error::Error for MissingOperand {}

fn int_type(width: IntType) -> Type {
    Type::Primitive(match width {
        IntType::U8 => PrimitiveType::U8,
        IntType::U16 => PrimitiveType::U16,
        IntType::U32 => PrimitiveType::U32,
        IntType::U64 => PrimitiveType::U64,
        IntType::U128 => PrimitiveType::U128,
        IntType::U256 => PrimitiveType::U256,
        IntType::I8 => PrimitiveType::I8,
        IntType::I16 => PrimitiveType::I16,
        IntType::I32 => PrimitiveType::I32,
        IntType::I64 => PrimitiveType::I64,
        IntType::I128 => PrimitiveType::I128,
        IntType::I256 => PrimitiveType::I256,
    })
}

fn bool_type() -> Type {
    Type::Primitive(PrimitiveType::Bool)
}

fn u64_type() -> Type {
    Type::Primitive(PrimitiveType::U64)
}

fn address_type() -> Type {
    Type::Primitive(PrimitiveType::Address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_model_exchange::{Field, TypeParameter as TypeParameterDecl, Variant};

    /// A module whose first function has one local of each type the rules
    /// distinguish, plus two callees and two more declarations:
    ///
    /// | local | type            | local | type              |
    /// |-------|-----------------|-------|-------------------|
    /// | 0, 1  | `u64`           | 13    | `&BalanceValue`   |
    /// | 2     | `u8`            | 14    | `&mut BalanceValue` |
    /// | 3, 4  | `bool`          | 15    | `&u64`            |
    /// | 5     | `address`       | 16    | `&mut u64`        |
    /// | 6     | `signer`        | 17    | `vector<u64>`     |
    /// | 7     | `&signer`       | 18    | `&vector<u64>`    |
    /// | 8     | `BalanceValue`  | 19    | `&mut vector<u64>` |
    /// | 9     | `Balance`       | 20    | `vector<bool>`    |
    /// | 10    | `G<u64>`        | 21    | `&E`              |
    /// | 11    | `G<bool>`       | 22    | `&mut Balance`    |
    /// | 12    | `E`             | 23    | `&mut signer`     |
    /// | 24    | `&Balance`      | 25    | `P<u64, bool>`    |
    /// | 26    | `GE<u64>`       | 27    | `&GE<u64>`        |
    /// | 28    | `R<u64>`        | 29    | `&R<u64>`         |
    /// | 30    | `P<u64>` (short) | 31   | `G` (no args)     |
    /// | 32    | `&G<u64>`       | 33    | `G<&u64>`         |
    ///
    /// Structs: 0 `BalanceValue { value: u64 }`, 1 `Balance { balance }` (the
    /// golden module), 2 `G<T> { x: T }`, 3 `enum E { A(u64), B(bool) }`,
    /// 4 `P<A, B>`, 5 `enum GE<T> { A(T), B(bool) }`, 6 `R<T: store> has key`.
    /// Functions: 1 `callee(u64, bool): u64`, 2 `id<T>(T): T`.
    fn module_with(instrs: Vec<Instr>, term: Term, returns: Vec<Ty>) -> XirModule {
        let golden = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xir/account.xir.json"),
        )
        .unwrap();
        let mut module: XirModule = serde_json::from_str(&golden).unwrap();
        let copy_drop = || vec!["copy".to_owned(), "drop".to_owned()];
        let field = |ty: Ty| Field {
            name: "x".to_owned(),
            ty,
        };
        module.structs.push(StructDecl {
            name: "G".to_owned(),
            abilities: copy_drop(),
            type_parameters: vec![TypeParameterDecl {
                name: "T".to_owned(),
                abilities: copy_drop(),
                phantom: false,
            }],
            fields: vec![field(Ty::TypeParameter(0))],
            variants: None,
            attributes: vec![],
        });
        module.structs.push(StructDecl {
            name: "E".to_owned(),
            abilities: copy_drop(),
            type_parameters: vec![],
            fields: vec![],
            variants: Some(vec![
                Variant {
                    name: "A".to_owned(),
                    fields: vec![field(Ty::U64)],
                },
                Variant {
                    name: "B".to_owned(),
                    fields: vec![field(Ty::Bool)],
                },
            ]),
            attributes: vec![],
        });
        let param = |name: &str, abilities: &[&str]| TypeParameterDecl {
            name: name.to_owned(),
            abilities: abilities.iter().map(|a| a.to_string()).collect(),
            phantom: false,
        };
        // 4: `P<A, B> { a: A, b: B }`.
        module.structs.push(StructDecl {
            name: "P".to_owned(),
            abilities: copy_drop(),
            type_parameters: vec![param("A", &["copy", "drop"]), param("B", &["copy", "drop"])],
            fields: vec![field(Ty::TypeParameter(0)), Field {
                name: "y".to_owned(),
                ty: Ty::TypeParameter(1),
            }],
            variants: None,
            attributes: vec![],
        });
        // 5: `enum GE<T> { A(T), B(bool) }`.
        module.structs.push(StructDecl {
            name: "GE".to_owned(),
            abilities: copy_drop(),
            type_parameters: vec![param("T", &["copy", "drop"])],
            fields: vec![],
            variants: Some(vec![
                Variant {
                    name: "A".to_owned(),
                    fields: vec![field(Ty::TypeParameter(0))],
                },
                Variant {
                    name: "B".to_owned(),
                    fields: vec![field(Ty::Bool)],
                },
            ]),
            attributes: vec![],
        });
        // 6: `R<T: store> has key { x: T }`.
        module.structs.push(StructDecl {
            name: "R".to_owned(),
            abilities: vec!["key".to_owned()],
            type_parameters: vec![param("T", &["store"])],
            fields: vec![field(Ty::TypeParameter(0))],
            variants: None,
            attributes: vec![],
        });
        let base = module.functions[0].clone();
        let function = |name: &str,
                        type_parameters: Vec<TypeParameterDecl>,
                        params: usize,
                        locals: Vec<Ty>,
                        returns: Vec<Ty>,
                        instrs: Vec<Instr>,
                        term: Term| {
            let mut f = base.clone();
            f.name = name.to_owned();
            f.type_parameters = type_parameters;
            f.params = params;
            f.locals = locals;
            f.local_names = vec![];
            f.returns = returns;
            f.acquires = vec![];
            f.is_entry = false;
            f.source_map = None;
            f.entry = 0;
            f.blocks = vec![Block { instrs, term }];
            f
        };
        let r = |t: Ty| Ty::Ref(Box::new(t));
        let m = |t: Ty| Ty::MutRef(Box::new(t));
        let v = |t: Ty| Ty::Vector(Box::new(t));
        let locals = vec![
            Ty::U64,
            Ty::U64,
            Ty::U8,
            Ty::Bool,
            Ty::Bool,
            Ty::Address,
            Ty::Signer,
            r(Ty::Signer),
            Ty::Struct(0),
            Ty::Struct(1),
            Ty::StructInst(2, vec![Ty::U64]),
            Ty::StructInst(2, vec![Ty::Bool]),
            Ty::Enum(3),
            r(Ty::Struct(0)),
            m(Ty::Struct(0)),
            r(Ty::U64),
            m(Ty::U64),
            v(Ty::U64),
            r(v(Ty::U64)),
            m(v(Ty::U64)),
            v(Ty::Bool),
            r(Ty::Enum(3)),
            m(Ty::Struct(1)),
            // 23..
            m(Ty::Signer),
            r(Ty::Struct(1)),
            Ty::StructInst(4, vec![Ty::U64, Ty::Bool]),
            Ty::StructInst(5, vec![Ty::U64]),
            r(Ty::StructInst(5, vec![Ty::U64])),
            Ty::StructInst(6, vec![Ty::U64]),
            r(Ty::StructInst(6, vec![Ty::U64])),
            // 30: `P` with one type argument; 31: `G` with none.
            Ty::StructInst(4, vec![Ty::U64]),
            Ty::Struct(2),
            r(Ty::StructInst(2, vec![Ty::U64])),
            // 33: `G<&u64>`.
            Ty::StructInst(2, vec![r(Ty::U64)]),
        ];
        module.functions = vec![
            function("f", vec![], 0, locals, returns, instrs, term),
            function(
                "callee",
                vec![],
                2,
                vec![Ty::U64, Ty::Bool],
                vec![Ty::U64],
                vec![],
                Term::Ret(vec![0]),
            ),
            function(
                "id",
                vec![TypeParameterDecl {
                    name: "T".to_owned(),
                    abilities: vec![],
                    phantom: false,
                }],
                1,
                vec![Ty::TypeParameter(0)],
                vec![Ty::TypeParameter(0)],
                vec![],
                Term::Ret(vec![0]),
            ),
        ];
        module
    }

    fn load(module: &XirModule) -> Result<()> {
        load_into(GlobalEnv::new(), module)
    }

    /// As [`load`], into a model holding the standard library: vector
    /// operations translate to calls into `0x1::vector`.
    fn load_with_stdlib(module: &XirModule) -> Result<()> {
        let options = crate::Options {
            dependencies: move_stdlib::move_stdlib_files(),
            named_address_mapping: vec!["std=0x1".to_owned()],
            ..crate::Options::default()
        };
        load_into(crate::run_checker(options)?, module)
    }

    fn load_into(mut env: GlobalEnv, module: &XirModule) -> Result<()> {
        let source = parse_source(
            std::path::PathBuf::from("typing.xir.json"),
            String::new(),
            &serde_json::to_string(module).unwrap(),
        )?;
        let mut targets = FunctionTargetsHolder::default();
        import_sources(&mut env, &[source], &mut targets)
    }

    fn call(dsts: &[usize], oper: Oper, srcs: &[usize]) -> Instr {
        Instr::Call(dsts.to_vec(), oper, srcs.to_vec())
    }

    /// Runs each case and lists those whose outcome differs from the expected
    /// one: `true` means the document must load, `false` that it must not.
    fn check(cases: Vec<(bool, Instr)>) {
        check_with(load, cases)
    }

    fn check_with(load: fn(&XirModule) -> Result<()>, cases: Vec<(bool, Instr)>) {
        let wrong: Vec<_> = cases
            .into_iter()
            .filter_map(|(ok, instr)| {
                let result = load(&module_with(vec![instr.clone()], Term::Ret(vec![]), vec![]));
                (result.is_ok() != ok).then(|| match result {
                    Ok(()) => format!("accepted {instr:?}"),
                    Err(error) => format!("rejected {instr:?}: {error:#}"),
                })
            })
            .collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn arithmetic_shift_and_cast() {
        check(vec![
            (true, call(&[0], Oper::Add(IntType::U64), &[0, 1])),
            (false, call(&[0], Oper::Add(IntType::U64), &[0, 2])),
            (false, call(&[2], Oper::Add(IntType::U64), &[0, 1])),
            (false, call(&[0], Oper::Add(IntType::U8), &[0, 1])),
            (true, call(&[0], Oper::Shl(IntType::U64), &[0, 2])),
            (false, call(&[0], Oper::Shl(IntType::U64), &[0, 1])),
            (false, call(&[0], Oper::Shr(IntType::U8), &[0, 2])),
            (true, call(&[2], Oper::Cast(IntType::U8), &[0])),
            (true, call(&[0], Oper::Cast(IntType::U64), &[2])),
            (false, call(&[2], Oper::Cast(IntType::U8), &[3])),
            (false, call(&[0], Oper::Cast(IntType::U8), &[0])),
        ]);
    }

    #[test]
    fn comparisons_and_booleans() {
        check(vec![
            (true, call(&[3], Oper::Lt, &[0, 1])),
            (false, call(&[3], Oper::Lt, &[0, 2])),
            (false, call(&[0], Oper::Le, &[0, 1])),
            (true, call(&[3], Oper::Le, &[0, 1])),
            (true, call(&[3], Oper::Le, &[2, 2])),
            (false, call(&[3], Oper::Le, &[0, 2])),
            (false, call(&[3], Oper::Le, &[8, 8])),   // struct
            (false, call(&[3], Oper::Le, &[5, 5])),   // address
            (false, call(&[3], Oper::Le, &[3, 4])),   // bool
            (false, call(&[3], Oper::Le, &[15, 15])), // reference
            (true, call(&[3], Oper::Eq, &[8, 8])),
            (false, call(&[3], Oper::Eq, &[8, 10])),
            (true, call(&[3], Oper::And, &[3, 4])),
            (false, call(&[3], Oper::Or, &[3, 0])),
            (true, call(&[3], Oper::Not, &[4])),
            (false, call(&[3], Oper::Not, &[0])),
        ]);
    }

    #[test]
    fn structs_and_generic_instantiation() {
        check(vec![
            (true, call(&[8], Oper::Pack, &[0])),
            (false, call(&[8], Oper::Pack, &[2])),
            (false, call(&[8], Oper::Pack, &[0, 1])),
            (true, call(&[10], Oper::PackInst(vec![Ty::U64]), &[0])),
            (false, call(&[10], Oper::PackInst(vec![Ty::Bool]), &[0])),
            (false, call(&[10], Oper::Pack, &[0])),
            (false, call(&[11], Oper::PackInst(vec![Ty::Bool]), &[0])),
            (true, call(&[0], Oper::Unpack, &[8])),
            (false, call(&[2], Oper::Unpack, &[8])),
            (true, call(&[0], Oper::GetField(0), &[8])),
            (false, call(&[2], Oper::GetField(0), &[8])),
            (false, call(&[0], Oper::GetField(0), &[13])),
            (true, call(&[15], Oper::BorrowField(0), &[13])),
            (true, call(&[16], Oper::BorrowField(0), &[14])),
            (true, call(&[15], Oper::BorrowField(0), &[14])),
            (false, call(&[16], Oper::BorrowField(0), &[13])),
            (false, call(&[15], Oper::BorrowField(0), &[8])),
        ]);
    }

    #[test]
    fn enums() {
        check(vec![
            (true, call(&[12], Oper::PackVariant(0), &[0])),
            (true, call(&[12], Oper::PackVariant(1), &[3])),
            (false, call(&[12], Oper::PackVariant(1), &[0])),
            (false, call(&[8], Oper::PackVariant(0), &[0])),
            (true, call(&[0], Oper::UnpackVariant(0), &[12])),
            (false, call(&[3], Oper::UnpackVariant(0), &[12])),
            (true, call(&[3], Oper::TestVariant(0), &[12])),
            (false, call(&[0], Oper::TestVariant(0), &[12])),
            (false, call(&[3], Oper::TestVariant(0), &[21])),
            (true, call(&[3], Oper::TestVariantRef(1), &[21])),
            (false, call(&[3], Oper::TestVariantRef(1), &[12])),
            (
                true,
                call(&[15], Oper::BorrowVariantField(vec![0], 0), &[21]),
            ),
            (
                false,
                call(&[15], Oper::BorrowVariantField(vec![0, 1], 0), &[21]),
            ),
        ]);
    }

    #[test]
    fn vectors() {
        check_with(load_with_stdlib, vec![
            (true, call(&[17], Oper::VecPack, &[0, 1])),
            (true, call(&[17], Oper::VecPack, &[])),
            (false, call(&[17], Oper::VecPack, &[0, 2])),
            (true, call(&[0], Oper::VecLen, &[17])),
            (true, call(&[0], Oper::VecLen, &[18])),
            (false, call(&[2], Oper::VecLen, &[17])),
            (false, call(&[0], Oper::VecLen, &[0])),
            (true, call(&[0], Oper::VecGet, &[17, 1])),
            (true, call(&[0], Oper::VecGet, &[18, 1])),
            (false, call(&[0], Oper::VecGet, &[17, 2])),
            (false, call(&[3], Oper::VecGet, &[17, 1])),
            (true, call(&[17], Oper::VecSet, &[17, 1, 0])),
            (false, call(&[17], Oper::VecSet, &[17, 1, 3])),
            (false, call(&[20], Oper::VecSet, &[17, 1, 0])),
            (true, call(&[17], Oper::VecPush, &[17, 0])),
            (false, call(&[17], Oper::VecPush, &[17, 3])),
            (true, call(&[17, 0], Oper::VecPop, &[17])),
            (false, call(&[17, 3], Oper::VecPop, &[17])),
            (true, call(&[17], Oper::VecInsert, &[17, 1, 0])),
            (false, call(&[17], Oper::VecInsert, &[17, 3, 0])),
            (true, call(&[17, 0], Oper::VecRemove, &[17, 1])),
            (false, call(&[17, 0], Oper::VecRemove, &[17, 3])),
            (true, call(&[17], Oper::VecSwap, &[17, 0, 1])),
            (false, call(&[17], Oper::VecSwap, &[17, 0, 2])),
            (true, call(&[15], Oper::BorrowVecElem, &[18, 1])),
            (true, call(&[16], Oper::BorrowVecElem, &[19, 1])),
            (false, call(&[16], Oper::BorrowVecElem, &[18, 1])),
            (false, call(&[15], Oper::BorrowVecElem, &[17, 1])),
        ]);
    }

    #[test]
    fn global_storage() {
        check(vec![
            (true, call(&[8], Oper::GetGlobal(0), &[5])),
            (false, call(&[8], Oper::GetGlobal(0), &[0])),
            (false, call(&[9], Oper::GetGlobal(0), &[5])),
            (true, call(&[], Oper::WriteGlobal(0), &[5, 8])),
            (false, call(&[], Oper::WriteGlobal(0), &[5, 9])),
            (true, call(&[], Oper::MoveTo(1), &[6, 9])),
            (true, call(&[], Oper::MoveTo(1), &[7, 9])),
            (false, call(&[], Oper::MoveTo(1), &[5, 9])),
            (false, call(&[], Oper::MoveTo(1), &[6, 8])),
            (true, call(&[9], Oper::MoveFrom(1), &[5])),
            (false, call(&[8], Oper::MoveFrom(1), &[5])),
            (true, call(&[3], Oper::Exists(1), &[5])),
            (false, call(&[0], Oper::Exists(1), &[5])),
            (true, call(&[22], Oper::BorrowGlobal(1), &[5])),
            (false, call(&[14], Oper::BorrowGlobal(1), &[5])),
        ]);
    }

    #[test]
    fn calls() {
        check(vec![
            (true, call(&[0], Oper::Function(1), &[0, 3])),
            (false, call(&[0], Oper::Function(1), &[0, 0])),
            (false, call(&[3], Oper::Function(1), &[0, 3])),
            (false, call(&[0], Oper::Function(1), &[0])),
            (true, call(&[0], Oper::FunctionInst(2, vec![Ty::U64]), &[1])),
            (
                false,
                call(&[3], Oper::FunctionInst(2, vec![Ty::U64]), &[1]),
            ),
            (false, call(&[0], Oper::Function(2), &[1])),
        ]);
    }

    #[test]
    fn references() {
        check(vec![
            (true, call(&[15], Oper::BorrowLoc, &[0])),
            (true, call(&[16], Oper::BorrowLoc, &[0])),
            (false, call(&[15], Oper::BorrowLoc, &[15])),
            (false, call(&[15], Oper::BorrowLoc, &[2])),
            (true, call(&[0], Oper::ReadRef, &[15])),
            (true, call(&[0], Oper::ReadRef, &[16])),
            (false, call(&[2], Oper::ReadRef, &[15])),
            (false, call(&[0], Oper::ReadRef, &[0])),
            (true, call(&[], Oper::WriteRef, &[16, 0])),
            (false, call(&[], Oper::WriteRef, &[15, 0])),
            (false, call(&[], Oper::WriteRef, &[16, 2])),
            (true, call(&[15], Oper::FreezeRef, &[16])),
            (false, call(&[15], Oper::FreezeRef, &[15])),
            (false, call(&[16], Oper::FreezeRef, &[16])),
        ]);
    }

    #[test]
    fn assign_and_load() {
        let num = || Constant::Num("7".to_owned());
        let address = || Constant::Address("0x1".to_owned());
        check(vec![
            (true, Instr::Assign(0, 1)),
            (false, Instr::Assign(0, 2)),
            (false, Instr::Assign(8, 10)),
            (true, Instr::Load(0, num())),
            (false, Instr::Load(3, num())),
            (true, Instr::Load(3, Constant::Bool(true))),
            (false, Instr::Load(0, Constant::Bool(true))),
            (true, Instr::Load(5, address())),
            (false, Instr::Load(0, address())),
        ]);
    }

    #[test]
    fn terminators() {
        let cases = [
            (true, Term::Branch(3, 0, 0), vec![]),
            (false, Term::Branch(0, 0, 0), vec![]),
            (true, Term::Abort(0), vec![]),
            (false, Term::Abort(2), vec![]),
            (true, Term::Ret(vec![0]), vec![Ty::U64]),
            (false, Term::Ret(vec![3]), vec![Ty::U64]),
        ];
        let wrong: Vec<_> = cases
            .into_iter()
            .filter_map(|(ok, term, returns)| {
                let result = load(&module_with(vec![], term.clone(), returns));
                (result.is_ok() != ok).then(|| format!("{term:?}: {result:?}"))
            })
            .collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// Every `Inst` operation, accepted with its operand's type arguments
    /// and rejected with different ones.
    #[test]
    fn instantiated_operations() {
        let u = || vec![Ty::U64];
        let b = || vec![Ty::Bool];
        check(vec![
            (true, call(&[0], Oper::UnpackInst(u()), &[10])),
            (false, call(&[0], Oper::UnpackInst(b()), &[10])),
            (false, call(&[0], Oper::Unpack, &[10])),
            (true, call(&[0], Oper::GetFieldInst(0, u()), &[10])),
            (false, call(&[0], Oper::GetFieldInst(0, b()), &[10])),
            (true, call(&[15], Oper::BorrowFieldInst(0, u()), &[32])),
            (false, call(&[15], Oper::BorrowFieldInst(0, b()), &[32])),
            (true, call(&[26], Oper::PackVariantInst(0, u()), &[0])),
            (false, call(&[26], Oper::PackVariantInst(0, b()), &[0])),
            (true, call(&[0], Oper::UnpackVariantInst(0, u()), &[26])),
            (false, call(&[0], Oper::UnpackVariantInst(0, b()), &[26])),
            (true, call(&[3], Oper::TestVariantInst(1, u()), &[26])),
            (false, call(&[3], Oper::TestVariantInst(1, b()), &[26])),
            (true, call(&[3], Oper::TestVariantRefInst(1, u()), &[27])),
            (false, call(&[3], Oper::TestVariantRefInst(1, b()), &[27])),
            (
                true,
                call(&[15], Oper::BorrowVariantFieldInst(vec![0], 0, u()), &[27]),
            ),
            (
                false,
                call(&[15], Oper::BorrowVariantFieldInst(vec![0], 0, b()), &[27]),
            ),
            (true, call(&[28], Oper::GetGlobalInst(6, u()), &[5])),
            (false, call(&[28], Oper::GetGlobalInst(6, b()), &[5])),
            (true, call(&[], Oper::MoveToInst(6, u()), &[6, 28])),
            (false, call(&[], Oper::MoveToInst(6, b()), &[6, 28])),
            (true, call(&[28], Oper::MoveFromInst(6, u()), &[5])),
            (false, call(&[28], Oper::MoveFromInst(6, b()), &[5])),
            (true, call(&[3], Oper::ExistsInst(6, u()), &[5])),
            (false, call(&[3], Oper::ExistsInst(6, vec![]), &[5])),
            (true, call(&[29], Oper::BorrowGlobalInst(6, u()), &[5])),
            (false, call(&[29], Oper::BorrowGlobalInst(6, b()), &[5])),
        ]);
    }

    /// A wrong number of type arguments is an error, not a panic, and a
    /// reference is not a type argument.
    #[test]
    fn type_argument_count_and_kind() {
        check(vec![
            (
                true,
                call(&[0, 3], Oper::UnpackInst(vec![Ty::U64, Ty::Bool]), &[25]),
            ),
            (false, call(&[0], Oper::UnpackInst(vec![Ty::U64]), &[30])),
            (false, call(&[31], Oper::Pack, &[0])),
            (
                false,
                call(&[3], Oper::ExistsInst(6, vec![Ty::U64, Ty::U64]), &[5]),
            ),
            (
                false,
                call(&[33], Oper::PackInst(vec![Ty::Ref(Box::new(Ty::U64))]), &[
                    15,
                ]),
            ),
            (
                false,
                call(
                    &[15],
                    Oper::FunctionInst(2, vec![Ty::Ref(Box::new(Ty::U64))]),
                    &[15],
                ),
            ),
        ]);
    }

    /// Accepted forms no other test reaches.
    #[test]
    fn remaining_forms() {
        check(vec![
            (true, call(&[], Oper::MoveTo(1), &[23, 9])),
            (true, call(&[24], Oper::BorrowGlobal(1), &[5])),
            (false, call(&[0], Oper::TestVariantRef(1), &[21])),
        ]);
    }

    #[test]
    fn a_rejection_names_the_position_and_both_types() {
        let error = load(&module_with(
            vec![call(&[0], Oper::Add(IntType::U64), &[0, 5])],
            Term::Ret(vec![]),
            vec![],
        ))
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains("instruction 0")
                && message.contains("operand 1 (l5)")
                && message.contains("`address`")
                && message.contains("`u64`"),
            "{message}"
        );
    }
}
