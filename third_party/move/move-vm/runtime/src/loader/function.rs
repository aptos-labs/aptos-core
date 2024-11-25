// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{
        access_specifier_loader::load_access_specifier, type_loader::intern_type, Module, Script,
    },
    native_functions::{NativeFunction, NativeFunctions, UnboxedNativeFunction},
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        AbilitySet, Bytecode, CompiledModule, CompiledScript, FunctionDefinitionIndex, Visibility,
    },
};
use move_core_types::{
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_types::loaded_data::{
    runtime_access_specifier::AccessSpecifier,
    runtime_types::{StructIdentifier, StructNameIndex, Type},
};
use std::{fmt::Debug, sync::Arc};

/// Type signature of a function, includes:
///   - parameter types,
///   - abilities of type parameters,
///   - access specifier (specifies where the function writes to or reads from in global state),
///   - return types.
pub(crate) struct FunctionSignature {
    pub(crate) param_tys: Vec<Type>,
    pub(crate) ty_param_abilities: Vec<AbilitySet>,
    pub(crate) access_specifier: AccessSpecifier,
    pub(crate) return_tys: Vec<Type>,
}

/// Represents internal details of a function. It can be either a native, or have a concrete
/// implementation with bytecode instructions.
pub(crate) enum FunctionInternals {
    Native {
        native: NativeFunction,
    },
    Bytecode {
        local_tys: Vec<Type>,
        bytecode: Vec<Bytecode>,
    },
}

/// A runtime representation for function definition.
pub struct Function {
    /// Index into file format representation corresponding to this definition.
    index: FunctionDefinitionIndex,
    /// The name of the function.
    name: Identifier,
    /// If true, this an entry function.
    is_entry: bool,
    /// If true, this function has private or public(friend) visibility.
    is_friend_or_private: bool,
    /// Function's type signature, including access specifier.
    signature: FunctionSignature,
    /// Internal function details, e.g., its bytecode or native implementation.
    internals: FunctionInternals,
}

impl Function {
    pub(crate) fn script_function(
        script: &CompiledScript,
        struct_name_indices: &[StructNameIndex],
    ) -> PartialVMResult<Self> {
        let params = script.signature_at(script.parameters);
        let binary_indexed_view = BinaryIndexedView::Script(script);

        let param_tys = params
            .0
            .iter()
            .map(|tok| intern_type(binary_indexed_view, tok, struct_name_indices))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let ty_param_abilities = script.type_parameters.clone();

        let local_tys = params
            .0
            .iter()
            .chain(script.signature_at(script.code.locals).0.iter())
            .map(|tok| intern_type(binary_indexed_view, tok, struct_name_indices))
            .collect::<PartialVMResult<Vec<_>>>()?;

        Ok(Function {
            index: FunctionDefinitionIndex(0),
            // TODO:
            //   Scripts do not have a name, we use "main" for now.
            name: ident_str!("main").to_owned(),
            // Script functions are not entry, and do not have visibility.
            is_entry: false,
            is_friend_or_private: false,
            signature: FunctionSignature {
                param_tys,
                ty_param_abilities,
                access_specifier: AccessSpecifier::Any,
                // Script must not return values.
                return_tys: vec![],
            },
            // Script entries cannot be native.
            internals: FunctionInternals::Bytecode {
                local_tys,
                bytecode: script.code.code.clone(),
            },
        })
    }

    pub(crate) fn module_function(
        natives: &NativeFunctions,
        index: FunctionDefinitionIndex,
        module: &CompiledModule,
        signature_table: &[Vec<Type>],
        struct_names: &[StructIdentifier],
    ) -> PartialVMResult<Self> {
        let function_def = module.function_def_at(index);
        let handle = module.function_handle_at(function_def.function);

        let name = module.identifier_at(handle.name).to_owned();
        let internals = if function_def.is_native() {
            assert!(function_def.code.is_none());

            let native = natives
                .resolve(
                    module.self_addr(),
                    module.self_name().as_str(),
                    name.as_str(),
                )
                .ok_or_else(|| {
                    let msg = format!(
                        "Native function {}::{}::{} does not exist",
                        module.self_addr(),
                        module.self_name(),
                        name
                    );
                    missing_native_function_error(msg)
                })?;
            FunctionInternals::Native { native }
        } else {
            let code_unit = function_def
                .code
                .as_ref()
                .expect("Code unit always exist for non-native functions");
            let bytecode = code_unit.code.clone();

            let mut local_tys = signature_table[handle.parameters.0 as usize].clone();
            local_tys.append(&mut signature_table[code_unit.locals.0 as usize].clone());

            FunctionInternals::Bytecode {
                local_tys,
                bytecode,
            }
        };

        let is_friend_or_private = match function_def.visibility {
            Visibility::Friend | Visibility::Private => true,
            Visibility::Public => false,
        };

        let param_tys = signature_table[handle.parameters.0 as usize].clone();
        let ty_param_abilities = handle.type_parameters.clone();
        let access_specifier = load_access_specifier(
            BinaryIndexedView::Module(module),
            signature_table,
            struct_names,
            &handle.access_specifiers,
        )?;
        let return_tys = signature_table[handle.return_.0 as usize].clone();

        Ok(Self {
            index,
            name,
            signature: FunctionSignature {
                param_tys,
                ty_param_abilities,
                access_specifier,
                return_tys,
            },
            internals,
            is_friend_or_private,
            is_entry: function_def.is_entry,
        })
    }

    pub fn name(&self) -> &IdentStr {
        self.name.as_ident_str()
    }

    pub fn param_tys(&self) -> &[Type] {
        &self.signature.param_tys
    }

    pub fn ty_param_abilities(&self) -> &[AbilitySet] {
        &self.signature.ty_param_abilities
    }

    pub fn return_tys(&self) -> &[Type] {
        &self.signature.return_tys
    }

    pub fn is_native(&self) -> bool {
        matches!(self.internals, FunctionInternals::Native { .. })
    }

    pub fn is_friend_or_private(&self) -> bool {
        self.is_friend_or_private
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("Function")
            .field("name", &self.name)
            .finish()
    }
}

/// For loaded function representation, specifies the owner: a script or a module.
pub(crate) enum LoadedFunctionOwner {
    Script(Arc<Script>),
    Module(Arc<Module>),
}

/// A runtime representation of a "loaded" function. In addition to function's definition, it
/// contains type arguments used to instantiate it and the "owner" of a function (either a script
/// or a module).
pub struct LoadedFunction {
    pub(crate) owner: LoadedFunctionOwner,
    /// A set of verified type arguments provided for this definition. If function is not generic,
    /// an empty vector.
    pub(crate) ty_args: Vec<Type>,
    /// Definition of the loaded function.
    pub(crate) function: Arc<Function>,
}

impl LoadedFunction {
    /// Returns type arguments used to instantiate the loaded function.
    pub fn ty_args(&self) -> &[Type] {
        &self.ty_args
    }

    /// Returns the corresponding module id of this function, i.e., its address and module name.
    pub fn module_id(&self) -> Option<&ModuleId> {
        match &self.owner {
            LoadedFunctionOwner::Module(m) => Some(m.self_id()),
            LoadedFunctionOwner::Script(_) => None,
        }
    }

    /// Returns the name of this function.
    pub fn name(&self) -> &IdentStr {
        self.function.name()
    }

    /// Returns true if the loaded function is an entry function.
    pub(crate) fn is_entry(&self) -> bool {
        self.function.is_entry
    }

    /// Returns true if the loaded function has private or public(friend) visibility.
    pub fn is_friend_or_private(&self) -> bool {
        self.function.is_friend_or_private()
    }

    /// Returns parameter types from the function's definition signature.
    pub fn param_tys(&self) -> &[Type] {
        self.function.param_tys()
    }

    /// Returns abilities of type parameters from the function's definition signature.
    pub fn ty_param_abilities(&self) -> &[AbilitySet] {
        self.function.ty_param_abilities()
    }

    /// Returns the access specifier for this function.
    pub(crate) fn access_specifier(&self) -> &AccessSpecifier {
        &self.function.signature.access_specifier
    }

    /// Returns return types from the function's definition signature.
    pub fn return_tys(&self) -> &[Type] {
        self.function.return_tys()
    }

    /// Returns types of locals, defined by this function. If the function is a native, returns an
    /// empty slice.
    pub fn local_tys(&self) -> &[Type] {
        match &self.function.internals {
            FunctionInternals::Native { .. } => &[],
            FunctionInternals::Bytecode { local_tys, .. } => local_tys,
        }
    }

    /// Returns true if this function is a native function, i.e. which does not contain
    /// code and instead using the Rust implementation.
    pub fn is_native(&self) -> bool {
        self.function.is_native()
    }

    /// If function is a native, returns its native implementation. Otherwise, returns an error.
    pub fn as_native_function(&self) -> PartialVMResult<&UnboxedNativeFunction> {
        match &self.function.internals {
            FunctionInternals::Native { native } => Ok(native.as_ref()),
            FunctionInternals::Bytecode { .. } => {
                let msg = format!(
                    "Native function {} must exist, but it does not",
                    self.function.name()
                );
                Err(missing_native_function_error(msg))
            },
        }
    }

    /// If function is a native, returns [None]. Otherwise, returns its bytecode.
    pub(crate) fn code(&self) -> Option<&[Bytecode]> {
        match &self.function.internals {
            FunctionInternals::Native { .. } => None,
            FunctionInternals::Bytecode { bytecode, .. } => Some(bytecode),
        }
    }

    /// Returns the function definition index into the file-format representation of a script or a
    /// module.
    pub(crate) fn index(&self) -> FunctionDefinitionIndex {
        self.function.index
    }

    pub(crate) fn name_as_pretty_string(&self) -> String {
        match &self.owner {
            LoadedFunctionOwner::Script(_) => "script::main".into(),
            LoadedFunctionOwner::Module(m) => format!(
                "0x{}::{}::{}",
                m.self_addr().to_hex(),
                m.self_name().as_str(),
                self.function.name()
            ),
        }
    }
}

//
// Internal structures that are saved at the proper index in the proper tables to access
// execution information (interpreter).
// The following structs are internal to the loader and never exposed out.
// The `Loader` will create those struct and the proper table when loading a module.
// The `Resolver` uses those structs to return information to the `Interpreter`.
//

// A function instantiation.
#[derive(Clone, Debug)]
pub(crate) struct FunctionInstantiation {
    pub(crate) handle: FunctionHandle,
    pub(crate) instantiation: Vec<Type>,
}

#[derive(Clone, Debug)]
pub(crate) enum FunctionHandle {
    Local(Arc<Function>),
    Remote { module: ModuleId, name: Identifier },
}

fn missing_native_function_error(msg: String) -> PartialVMError {
    PartialVMError::new(StatusCode::MISSING_DEPENDENCY).with_message(msg)
}
