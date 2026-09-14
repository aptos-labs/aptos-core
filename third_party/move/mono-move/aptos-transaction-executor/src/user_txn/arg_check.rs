// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checking a payload's parameter types and BCS arguments against what a
//! transaction may construct, before any argument is placed.

use crate::errors::{invariant_violation, InvalidArguments, MoveExecutionFailure};
use mono_move_core::{
    intern_type_tag,
    interner::{view_module_id, InternedIdentifier, InternedModuleId},
    types::{view_name, view_type, view_type_list, InternedType, InternedTypeList, Type},
    FieldTypes, Interner, PreparedModule, VMInternalError,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{read_slice, read_uleb128_len, InterpreterContext, RuntimeError};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionAttribute, StructFieldInformation, VariantIndex, Visibility},
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{StructTag, TypeTag},
};

/// The most object existence checks one argument may make.
const MAX_OBJECT_CHECKS_PER_ARG: usize = 32;

/// Checks that every parameter type can be filled by a transaction argument.
/// - Primitives, and vectors of allowed types.
/// - `String`, `Object<T>`, `FixedPoint32`, `FixedPoint64`, and `Option<E>`
///   of an allowed `E`.
/// - Public structs and enums whose fields are allowed.
pub(super) fn check_param_types<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    param_tys: &[InternedType],
) -> Result<(), MoveExecutionFailure> {
    let mut checker = ArgChecker::new(guard, interp)?;
    for &ty in param_tys {
        if !checker.is_allowed(ty)? {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::DisallowedParameterType,
            ));
        }
    }
    Ok(())
}

/// Checks that each argument holds a value the framework's argument
/// constructors would accept, and nothing more.
/// - A `String` is valid UTF-8.
/// - An `Option` holds at most one value.
/// - An `Object<T>` names an address holding an `ObjectCore` and a `T`.
/// - An enum's tag names one of its variants.
///
/// `param_tys` are the non-signer parameter types, one per argument, already
/// checked by `check_param_types`.
pub(super) fn check_arg_values<'a>(
    guard: &ExecutionGuard<'a>,
    interp: &mut InterpreterContext<'a>,
    param_tys: &[InternedType],
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    let mut checker = ArgChecker::new(guard, interp)?;
    for (&ty, arg) in param_tys.iter().zip(args) {
        let mut walk = Walk {
            bytes: arg,
            cursor: 0,
            object_checks: 0,
        };
        checker.walk(&mut walk, ty)?;
        if walk.cursor != arg.len() {
            return Err(undecodable());
        }
    }
    Ok(())
}

/// The framework types a transaction may construct.
enum Framework {
    String,
    Option,
    Object,
    FixedPoint32,
    FixedPoint64,
}

/// The framework type `module_id::name` is, if any.
fn framework_type(module_id: InternedModuleId, name: InternedIdentifier) -> Option<Framework> {
    let module_id = view_module_id(module_id);
    if *module_id.address() != AccountAddress::ONE {
        return None;
    }
    match (view_name(module_id.name()), view_name(name)) {
        ("string", "String") => Some(Framework::String),
        ("option", "Option") => Some(Framework::Option),
        ("object", "Object") => Some(Framework::Object),
        ("fixed_point32", "FixedPoint32") => Some(Framework::FixedPoint32),
        ("fixed_point64", "FixedPoint64") => Some(Framework::FixedPoint64),
        _ => None,
    }
}

/// The instantiated field types of a struct, or of each variant of an enum in
/// tag order.
enum Fields {
    Struct(Vec<InternedType>),
    Enum(Vec<Vec<InternedType>>),
}

/// Checks types and values against the transaction's own interpreter, which
/// loads the modules they name.
struct ArgChecker<'w, 'g> {
    guard: &'w ExecutionGuard<'g>,
    interp: &'w mut InterpreterContext<'g>,
    object_core: InternedType,
}

impl<'w, 'g> ArgChecker<'w, 'g> {
    fn new(
        guard: &'w ExecutionGuard<'g>,
        interp: &'w mut InterpreterContext<'g>,
    ) -> Result<Self, MoveExecutionFailure> {
        let tag = TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("object").to_owned(),
            name: ident_str!("ObjectCore").to_owned(),
            type_args: vec![],
        }));
        let object_core = intern_type_tag(&tag, guard).map_err(|err| {
            MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string()))
        })?;
        Ok(Self {
            guard,
            interp,
            object_core,
        })
    }

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    /// Whether a transaction argument can fill a parameter of type `ty`.
    fn is_allowed(&mut self, ty: InternedType) -> Result<bool, MoveExecutionFailure> {
        match view_type(ty) {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address => Ok(true),
            Type::Vector { elem } => self.is_allowed(*elem),
            Type::Nominal {
                module_id,
                name,
                ty_args,
            } => match framework_type(*module_id, *name) {
                Some(Framework::Option) => self.is_allowed(type_arg(*ty_args)?),
                // An `Object<T>` argument is only an address, so `T` is unrestricted.
                Some(
                    Framework::String
                    | Framework::Object
                    | Framework::FixedPoint32
                    | Framework::FixedPoint64,
                ) => Ok(true),
                None => self.is_public_struct(*module_id, *name, *ty_args),
            },
            Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => Ok(false),
        }
    }

    /// Whether `S<A..>` is a public struct or enum a transaction may construct.
    /// - Its definition declares `copy` and not `key`.
    /// - Its module has a public pack function for it, or one for every
    ///   variant.
    /// - Its field types, instantiated with `A..`, are themselves allowed.
    fn is_public_struct(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Result<bool, MoveExecutionFailure> {
        let module = self
            .interp
            .load_module(module_id)
            .map_err(MoveExecutionFailure::RuntimeError)?;
        let Some(handle) = module.nominal_handle(module_id, name) else {
            return Ok(false);
        };
        if !handle.abilities.has_copy() || handle.abilities.has_key() {
            return Ok(false);
        }
        let Some(def_idx) = module.interned_nominal_type_def_idx(name) else {
            return Ok(false);
        };
        let struct_name = view_name(name);
        let has_pack_fns = match &module.struct_def_at(def_idx).field_information {
            StructFieldInformation::Declared(_) => has_public_pack_fn(
                module,
                &format!("pack${struct_name}"),
                FunctionAttribute::Pack,
            ),
            StructFieldInformation::DeclaredVariants(variants) => {
                variants.iter().enumerate().all(|(tag, variant)| {
                    has_public_pack_fn(
                        module,
                        &format!("pack${struct_name}${}", module.identifier_at(variant.name)),
                        FunctionAttribute::PackVariant(tag as VariantIndex),
                    )
                })
            },
            StructFieldInformation::Native => false,
        };
        if !has_pack_fns {
            return Ok(false);
        }
        let fields = match self.fields_of(module, name, ty_args)? {
            Fields::Struct(fields) => fields,
            Fields::Enum(variants) => variants.into_iter().flatten().collect(),
        };
        for ty in fields {
            if !self.is_allowed(ty)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The field types of `name`, defined in `module`, instantiated with
    /// `ty_args`.
    fn fields_of(
        &self,
        module: &PreparedModule,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Result<Fields, MoveExecutionFailure> {
        let subst = |ty: InternedType| {
            self.guard.subst_type(ty, ty_args).map_err(|err| {
                MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string()))
            })
        };
        match module.interned_field_types(name) {
            Some(FieldTypes::Struct(fields)) => Ok(Fields::Struct(
                fields
                    .iter()
                    .map(|&ty| subst(ty))
                    .collect::<Result<_, _>>()?,
            )),
            Some(FieldTypes::Enum(variants)) => Ok(Fields::Enum(
                variants
                    .iter()
                    .map(|fields| fields.iter().map(|&ty| subst(ty)).collect())
                    .collect::<Result<_, _>>()?,
            )),
            None => Err(MoveExecutionFailure::RuntimeError(invariant_violation(
                "nominal type without a definition in its module",
            ))),
        }
    }

    // -----------------------------------------------------------------------
    // Values
    // -----------------------------------------------------------------------

    /// Walks one value of type `ty`, checking what it holds.
    fn walk(&mut self, w: &mut Walk<'_>, ty: InternedType) -> Result<(), MoveExecutionFailure> {
        match view_type(ty) {
            Type::Bool | Type::U8 | Type::I8 => w.skip(1),
            Type::U16 | Type::I16 => w.skip(2),
            Type::U32 | Type::I32 => w.skip(4),
            Type::U64 | Type::I64 => w.skip(8),
            Type::U128 | Type::I128 => w.skip(16),
            Type::U256 | Type::I256 | Type::Address => w.skip(32),
            Type::Vector { elem } => {
                let len = w.read_len()?;
                if let Some(size) = primitive_size(*elem) {
                    let bytes = len.checked_mul(size).ok_or_else(undecodable)?;
                    return w.skip(bytes);
                }
                for _ in 0..len {
                    self.walk(w, *elem)?;
                }
                Ok(())
            },
            Type::Nominal {
                module_id,
                name,
                ty_args,
            } => match framework_type(*module_id, *name) {
                Some(Framework::String) => {
                    let len = w.read_len()?;
                    if std::str::from_utf8(w.read(len)?).is_err() {
                        return Err(MoveExecutionFailure::InvalidArguments(
                            InvalidArguments::MalformedString,
                        ));
                    }
                    Ok(())
                },
                Some(Framework::Option) => match w.read_len()? {
                    0 => Ok(()),
                    1 => self.walk(w, type_arg(*ty_args)?),
                    _ => Err(undecodable()),
                },
                Some(Framework::Object) => {
                    let address = AccountAddress::from_bytes(w.read(AccountAddress::LENGTH)?)
                        .map_err(|err| {
                            MoveExecutionFailure::RuntimeError(invariant_violation(err.to_string()))
                        })?;
                    self.check_object(w, address, type_arg(*ty_args)?)
                },
                Some(Framework::FixedPoint32) => w.skip(8),
                Some(Framework::FixedPoint64) => w.skip(16),
                None => {
                    let module = self
                        .interp
                        .load_module(*module_id)
                        .map_err(MoveExecutionFailure::RuntimeError)?;
                    let fields = match self.fields_of(module, *name, *ty_args)? {
                        Fields::Struct(fields) => fields,
                        Fields::Enum(mut variants) => {
                            let tag = w.read_len()?;
                            if tag >= variants.len() {
                                return Err(undecodable());
                            }
                            variants.swap_remove(tag)
                        },
                    };
                    for ty in fields {
                        self.walk(w, ty)?;
                    }
                    Ok(())
                },
            },
            // The type check refuses these parameter types.
            Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => Err(MoveExecutionFailure::RuntimeError(
                invariant_violation("argument of a type a transaction cannot fill"),
            )),
        }
    }

    /// Checks that `address` holds an object and a `resource`, like
    /// `object::address_to_object` does.
    fn check_object(
        &mut self,
        w: &mut Walk<'_>,
        address: AccountAddress,
        resource: InternedType,
    ) -> Result<(), MoveExecutionFailure> {
        if w.object_checks == MAX_OBJECT_CHECKS_PER_ARG {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::TooManyObjectChecks,
            ));
        }
        w.object_checks += 1;
        // TODO(metering): charge for the two reads.
        if !self
            .interp
            .resource_exists(address, self.object_core)
            .map_err(MoveExecutionFailure::RuntimeError)?
        {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::ObjectDoesNotExist,
            ));
        }
        if !self
            .interp
            .resource_exists(address, resource)
            .map_err(MoveExecutionFailure::RuntimeError)?
        {
            return Err(MoveExecutionFailure::InvalidArguments(
                InvalidArguments::ObjectLacksResource,
            ));
        }
        Ok(())
    }
}

/// Whether `module` defines a public function `name` carrying `attribute`.
fn has_public_pack_fn(module: &PreparedModule, name: &str, attribute: FunctionAttribute) -> bool {
    module.function_defs().iter().any(|def| {
        let handle = module.function_handle_at(def.function);
        def.visibility == Visibility::Public
            && module.identifier_at(handle.name).as_str() == name
            && handle.attributes.contains(&attribute)
    })
}

/// The cursor over one argument's bytes.
struct Walk<'b> {
    bytes: &'b [u8],
    cursor: usize,
    object_checks: usize,
}

impl<'b> Walk<'b> {
    /// Reads a BCS length or enum tag.
    fn read_len(&mut self) -> Result<usize, MoveExecutionFailure> {
        let len = read_uleb128_len(self.bytes, &mut self.cursor).map_err(decode_error)?;
        if len > bcs::MAX_SEQUENCE_LENGTH as u64 {
            return Err(undecodable());
        }
        Ok(len as usize)
    }

    fn read(&mut self, n: usize) -> Result<&'b [u8], MoveExecutionFailure> {
        read_slice(self.bytes, &mut self.cursor, n).map_err(decode_error)
    }

    fn skip(&mut self, n: usize) -> Result<(), MoveExecutionFailure> {
        self.read(n).map(|_| ())
    }
}

/// The one type argument of a framework type that takes one.
fn type_arg(ty_args: InternedTypeList) -> Result<InternedType, MoveExecutionFailure> {
    view_type_list(ty_args).first().copied().ok_or_else(|| {
        MoveExecutionFailure::RuntimeError(invariant_violation(
            "framework argument type without its type argument",
        ))
    })
}

/// The BCS size of a primitive type, or `None` for any other type.
fn primitive_size(ty: InternedType) -> Option<usize> {
    match view_type(ty) {
        Type::Bool | Type::U8 | Type::I8 => Some(1),
        Type::U16 | Type::I16 => Some(2),
        Type::U32 | Type::I32 => Some(4),
        Type::U64 | Type::I64 => Some(8),
        Type::U128 | Type::I128 => Some(16),
        Type::U256 | Type::I256 | Type::Address => Some(32),
        Type::Vector { .. }
        | Type::Nominal { .. }
        | Type::Signer
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => None,
    }
}

fn undecodable() -> MoveExecutionFailure {
    MoveExecutionFailure::InvalidArguments(InvalidArguments::UndecodableArgument)
}

/// Only a decode failure faults the argument's bytes; anything else is the VM's.
fn decode_error(err: RuntimeError) -> MoveExecutionFailure {
    if err.is_bcs_decode_error() {
        undecodable()
    } else {
        MoveExecutionFailure::RuntimeError(VMInternalError::new(err))
    }
}
