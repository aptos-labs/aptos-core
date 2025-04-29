// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{access_specifier_loader::load_access_specifier, Module, Script},
    native_functions::{NativeFunction, NativeFunctions, UnboxedNativeFunction},
    storage::ty_tag_converter::TypeTagConverter,
    LayoutConverter, ModuleStorage, StorageLayoutConverter,
};
use better_any::{Tid, TidAble, TidExt};
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CompiledModule, FunctionAttribute, FunctionDefinitionIndex, Visibility,
    },
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    function::ClosureMask,
    identifier::{IdentStr, Identifier},
    language_storage,
    language_storage::{ModuleId, TypeTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    loaded_data::{
        runtime_access_specifier::AccessSpecifier,
        runtime_types::{StructIdentifier, Type},
    },
    values::{AbstractFunction, SerializedFunctionData},
};
use std::{cell::RefCell, cmp::Ordering, fmt::Debug, rc::Rc, sync::Arc};

/// A runtime function definition representation.
pub struct Function {
    #[allow(unused)]
    pub(crate) file_format_version: u32,
    pub(crate) index: FunctionDefinitionIndex,
    pub(crate) code: Vec<Bytecode>,
    pub(crate) ty_param_abilities: Vec<AbilitySet>,
    // TODO: Make `native` and `def_is_native` become an enum.
    pub(crate) native: Option<NativeFunction>,
    pub(crate) is_native: bool,
    pub(crate) is_friend_or_private: bool,
    pub(crate) is_entry: bool,
    pub(crate) name: Identifier,
    pub(crate) return_tys: Vec<Type>,
    pub(crate) local_tys: Vec<Type>,
    pub(crate) param_tys: Vec<Type>,
    pub(crate) access_specifier: AccessSpecifier,
    pub(crate) is_persistent: bool,
    pub(crate) has_module_reentrancy_lock: bool,
}

/// For loaded function representation, specifies the owner: a script or a module.
#[derive(Clone)]
pub enum LoadedFunctionOwner {
    Script(Arc<Script>),
    Module(Arc<Module>),
}

/// A loaded runtime function representation along with type arguments used to instantiate it.
#[derive(Clone)]
pub struct LoadedFunction {
    pub owner: LoadedFunctionOwner,
    // A set of verified type arguments provided for this definition. If
    // function is not generic, an empty vector.
    pub ty_args: Vec<Type>,
    // Definition of the loaded function.
    pub function: Arc<Function>,
}

impl LoadedFunction {
    pub fn owner(&self) -> &LoadedFunctionOwner {
        &self.owner
    }
}

/// A lazy loaded function, which can either be unresolved (as resulting
/// from deserialization) or resolved, and then forwarding to a
/// `LoadedFunction`. This is wrapped into a Rc so one can clone the
/// function while sharing the loading state.
#[derive(Clone, Tid)]
pub(crate) struct LazyLoadedFunction(pub(crate) Rc<RefCell<LazyLoadedFunctionState>>);

#[derive(Clone)]
pub(crate) enum LazyLoadedFunctionState {
    Unresolved {
        data: SerializedFunctionData,
    },
    Resolved {
        fun: Rc<LoadedFunction>,
        // For a resolved function, we need to store the type argument tags,
        // even though we have the resolved `Type` for the arguments in `fun.ty_args`.
        // This is needed so we can compare with deterministic results an unresolved and
        // resolved function context free (i.e. wo/ converter from Type to TypeTag). For the
        // unresolved case, the type argument tags are stored with the serialized data.
        ty_args: Vec<TypeTag>,
        mask: ClosureMask,
    },
}

impl LazyLoadedFunction {
    pub(crate) fn new_unresolved(data: SerializedFunctionData) -> Self {
        Self(Rc::new(RefCell::new(LazyLoadedFunctionState::Unresolved {
            data,
        })))
    }

    pub(crate) fn new_resolved(
        converter: &TypeTagConverter,
        fun: Rc<LoadedFunction>,
        mask: ClosureMask,
    ) -> PartialVMResult<Self> {
        let ty_args = fun
            .ty_args
            .iter()
            .map(|t| converter.ty_to_ty_tag(t))
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok(Self(Rc::new(RefCell::new(
            LazyLoadedFunctionState::Resolved { fun, ty_args, mask },
        ))))
    }

    pub(crate) fn expect_this_impl(
        fun: &dyn AbstractFunction,
    ) -> PartialVMResult<&LazyLoadedFunction> {
        fun.downcast_ref::<LazyLoadedFunction>().ok_or_else(|| {
            PartialVMError::new_invariant_violation("unexpected abstract function implementation")
        })
    }

    /// Access name components independent of resolution state. Since RefCell is in the play,
    /// the accessor is passed in as a function.
    ///
    /// Notice if no module id is given to the action, the function being processed stems from
    /// a script.
    pub(crate) fn with_name_and_ty_args<T>(
        &self,
        action: impl FnOnce(Option<&ModuleId>, &IdentStr, &[TypeTag]) -> T,
    ) -> T {
        match &*self.0.borrow() {
            LazyLoadedFunctionState::Unresolved {
                data:
                    SerializedFunctionData {
                        module_id,
                        fun_id,
                        ty_args,
                        ..
                    },
                ..
            } => action(Some(module_id), fun_id, ty_args),
            LazyLoadedFunctionState::Resolved { fun, ty_args, .. } => {
                action(fun.module_id(), fun.name_id(), ty_args)
            },
        }
    }

    /// Executed an action with the resolved loaded function. If the function hasn't been
    /// loaded yet, it will be loaded now.
    #[allow(unused)]
    pub(crate) fn with_resolved_function<T>(
        &self,
        storage: &dyn ModuleStorage,
        action: impl FnOnce(Rc<LoadedFunction>) -> PartialVMResult<T>,
    ) -> PartialVMResult<T> {
        let mut state = self.0.borrow_mut();
        match &mut *state {
            LazyLoadedFunctionState::Resolved { fun, .. } => action(fun.clone()),
            LazyLoadedFunctionState::Unresolved {
                data:
                    SerializedFunctionData {
                        format_version: _,
                        module_id,
                        fun_id,
                        ty_args,
                        mask,
                        captured_layouts,
                    },
            } => {
                let fun =
                    Self::resolve(storage, module_id, fun_id, ty_args, *mask, captured_layouts)?;
                let result = action(fun.clone());
                *state = LazyLoadedFunctionState::Resolved {
                    fun,
                    ty_args: ty_args.clone(),
                    mask: *mask,
                };
                result
            },
        }
    }

    /// Resolves a function into a loaded function. This verifies existence of the named
    /// function as well as whether it has the type used for deserializing the captured values.
    fn resolve(
        module_storage: &dyn ModuleStorage,
        module_id: &ModuleId,
        fun_id: &IdentStr,
        ty_args: &[TypeTag],
        mask: ClosureMask,
        captured_layouts: &[MoveTypeLayout],
    ) -> PartialVMResult<Rc<LoadedFunction>> {
        let function = module_storage
            .load_function(module_id, fun_id, ty_args)
            .map_err(|err| err.to_partial())?;

        // Verify that the function argument types match the layouts used for deserialization.
        // This is only done in paranoid mode. Since integrity of storage
        // and guarantee of public function, this should not able to fail.
        if module_storage
            .runtime_environment()
            .vm_config()
            .paranoid_type_checks
        {
            // TODO(#15664): Determine whether we need to charge gas here.
            let captured_arg_types = mask.extract(function.param_tys(), true);
            let converter = StorageLayoutConverter::new(module_storage);
            if captured_arg_types.len() != captured_layouts.len() {
                return Err(PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                    .with_message(
                        "captured argument count does not match declared parameters".to_string(),
                    ));
            }

            let ty_builder = &module_storage.runtime_environment().vm_config().ty_builder;
            for (actual_arg_ty, serialized_layout) in
                captured_arg_types.into_iter().zip(captured_layouts)
            {
                // Note that the below call returns a runtime layout, so we can directly
                // compare it without desugaring.
                let actual_arg_layout = if function.ty_args().is_empty() {
                    converter.type_to_type_layout(actual_arg_ty)?
                } else {
                    let actual_arg_ty =
                        ty_builder.create_ty_with_subst(actual_arg_ty, function.ty_args())?;
                    converter.type_to_type_layout(&actual_arg_ty)?
                };
                if !serialized_layout.is_compatible_with(&actual_arg_layout) {
                    return Err(PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                        .with_message(
                            "stored captured argument layout does not match declared parameters"
                                .to_string(),
                        ));
                }
            }
        }
        Ok(Rc::new(function))
    }
}

impl AbstractFunction for LazyLoadedFunction {
    fn closure_mask(&self) -> ClosureMask {
        let state = self.0.borrow();
        match &*state {
            LazyLoadedFunctionState::Resolved { mask, .. } => *mask,
            LazyLoadedFunctionState::Unresolved {
                data: SerializedFunctionData { mask, .. },
                ..
            } => *mask,
        }
    }

    fn cmp_dyn(&self, other: &dyn AbstractFunction) -> PartialVMResult<Ordering> {
        let other = LazyLoadedFunction::expect_this_impl(other)?;
        self.with_name_and_ty_args(|mid1, fid1, inst1| {
            other.with_name_and_ty_args(|mid2, fid2, inst2| {
                Ok(mid1
                    .cmp(&mid2)
                    .then_with(|| fid1.cmp(fid2))
                    .then_with(|| inst1.cmp(inst2)))
            })
        })
    }

    fn clone_dyn(&self) -> PartialVMResult<Box<dyn AbstractFunction>> {
        Ok(Box::new(self.clone()))
    }

    fn to_stable_string(&self) -> String {
        self.with_name_and_ty_args(|module_id, fun_id, ty_args| {
            let prefix = if let Some(m) = module_id {
                format!("0x{}::{}::", m.address(), m.name())
            } else {
                "".to_string()
            };
            let ty_args_str = if ty_args.is_empty() {
                "".to_string()
            } else {
                format!(
                    "<{}>",
                    ty_args
                        .iter()
                        .map(|t| t.to_canonical_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            format!("{}::{}:{}", prefix, fun_id, ty_args_str)
        })
    }
}

impl LoadedFunction {
    /// Returns type arguments used to instantiate the loaded function.
    pub fn ty_args(&self) -> &[Type] {
        &self.ty_args
    }

    pub fn abilities(&self) -> AbilitySet {
        self.function.abilities()
    }

    /// Returns the corresponding module id of this function, i.e., its address and module name.
    pub fn module_id(&self) -> Option<&ModuleId> {
        match &self.owner {
            LoadedFunctionOwner::Module(m) => Some(Module::self_id(m)),
            LoadedFunctionOwner::Script(_) => None,
        }
    }

    /// Returns the module id or, if it is a script, the pseudo module id for scripts.
    pub fn module_or_script_id(&self) -> &ModuleId {
        match &self.owner {
            LoadedFunctionOwner::Module(m) => Module::self_id(m),
            LoadedFunctionOwner::Script(_) => language_storage::pseudo_script_module_id(),
        }
    }

    /// Returns the name of this function.
    pub fn name(&self) -> &str {
        self.function.name()
    }

    /// Returns the id of this function's name.
    pub fn name_id(&self) -> &IdentStr {
        self.function.name_id()
    }

    /// Returns true if the loaded function has friend or private visibility.
    pub fn is_friend_or_private(&self) -> bool {
        self.function.is_friend_or_private()
    }

    /// Returns true if the loaded function has public visibility. This is the
    /// opposite of the above (for better readability).
    pub fn is_public(&self) -> bool {
        !self.function.is_friend_or_private()
    }

    /// Returns true if the loaded function is an entry function.
    pub fn is_entry(&self) -> bool {
        self.function.is_entry()
    }

    /// Returns parameter types from the function's definition signature.
    pub fn param_tys(&self) -> &[Type] {
        self.function.param_tys()
    }

    /// Returns return types from the function's definition signature.
    pub fn return_tys(&self) -> &[Type] {
        self.function.return_tys()
    }

    /// Returns abilities of type parameters from the function's definition signature.
    pub fn ty_param_abilities(&self) -> &[AbilitySet] {
        self.function.ty_param_abilities()
    }

    /// Returns types of locals, defined by this function.
    pub fn local_tys(&self) -> &[Type] {
        self.function.local_tys()
    }

    /// Returns true if this function is a native function, i.e. which does not contain
    /// code and instead using the Rust implementation.
    pub fn is_native(&self) -> bool {
        self.function.is_native()
    }

    pub fn get_native(&self) -> PartialVMResult<&UnboxedNativeFunction> {
        self.function.get_native()
    }

    pub(crate) fn index(&self) -> FunctionDefinitionIndex {
        self.function.index
    }

    pub(crate) fn code(&self) -> &[Bytecode] {
        &self.function.code
    }

    pub(crate) fn code_size(&self) -> usize {
        self.function.code.len()
    }

    pub(crate) fn access_specifier(&self) -> &AccessSpecifier {
        &self.function.access_specifier
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

impl Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("Function")
            .field("name", &self.name)
            .finish()
    }
}

impl Function {
    pub(crate) fn new(
        natives: &NativeFunctions,
        index: FunctionDefinitionIndex,
        module: &CompiledModule,
        signature_table: &[Vec<Type>],
        struct_names: &[StructIdentifier],
    ) -> PartialVMResult<Self> {
        let def = module.function_def_at(index);
        let handle = module.function_handle_at(def.function);
        let name = module.identifier_at(handle.name).to_owned();
        let module_id = module.self_id();

        let is_friend_or_private = match def.visibility {
            Visibility::Friend | Visibility::Private => true,
            Visibility::Public => false,
        };
        let is_entry = def.is_entry;

        let (native, is_native) = if def.is_native() {
            let native = natives.resolve(
                module_id.address(),
                module_id.name().as_str(),
                name.as_str(),
            );
            (native, true)
        } else {
            (None, false)
        };
        // Native functions do not have a code unit
        let code = match &def.code {
            Some(code) => code.code.clone(),
            None => vec![],
        };
        let ty_param_abilities = handle.type_parameters.clone();
        let return_tys = signature_table[handle.return_.0 as usize].clone();
        let local_tys = if let Some(code) = &def.code {
            let mut local_tys = signature_table[handle.parameters.0 as usize].clone();
            local_tys.append(&mut signature_table[code.locals.0 as usize].clone());
            local_tys
        } else {
            vec![]
        };
        let param_tys = signature_table[handle.parameters.0 as usize].clone();

        let access_specifier = load_access_specifier(
            BinaryIndexedView::Module(module),
            signature_table,
            struct_names,
            &handle.access_specifiers,
        )?;

        Ok(Self {
            file_format_version: module.version(),
            index,
            code,
            ty_param_abilities,
            native,
            is_native,
            is_friend_or_private,
            is_entry,
            name,
            local_tys,
            return_tys,
            param_tys,
            access_specifier,
            is_persistent: handle.attributes.contains(&FunctionAttribute::Persistent),
            has_module_reentrancy_lock: handle.attributes.contains(&FunctionAttribute::ModuleLock),
        })
    }

    #[allow(unused)]
    pub(crate) fn file_format_version(&self) -> u32 {
        self.file_format_version
    }

    pub fn param_count(&self) -> usize {
        self.param_tys.len()
    }

    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn name_id(&self) -> &IdentStr {
        &self.name
    }

    pub fn ty_param_abilities(&self) -> &[AbilitySet] {
        &self.ty_param_abilities
    }

    pub(crate) fn local_tys(&self) -> &[Type] {
        &self.local_tys
    }

    pub fn return_tys(&self) -> &[Type] {
        &self.return_tys
    }

    pub fn param_tys(&self) -> &[Type] {
        &self.param_tys
    }

    pub fn is_persistent(&self) -> bool {
        self.is_persistent || !self.is_friend_or_private()
    }

    pub fn has_module_lock(&self) -> bool {
        self.has_module_reentrancy_lock
    }

    /// Creates the function type instance for this function. This requires cloning
    /// the parameter and result types.
    pub fn create_function_type(&self) -> Type {
        Type::Function {
            args: self.param_tys.clone(),
            results: self.return_tys.clone(),
            abilities: self.abilities(),
        }
    }

    /// Returns the abilities associated with this function, without consideration of any captured
    /// closure arguments. By default, this is copy and drop, and if the function is
    /// immutable (public), also store.
    pub fn abilities(&self) -> AbilitySet {
        let result = AbilitySet::singleton(Ability::Copy).add(Ability::Drop);
        if !self.is_friend_or_private {
            result.add(Ability::Store)
        } else {
            result
        }
    }

    pub fn is_native(&self) -> bool {
        self.is_native
    }

    pub fn is_friend_or_private(&self) -> bool {
        self.is_friend_or_private
    }

    pub(crate) fn is_entry(&self) -> bool {
        self.is_entry
    }

    pub(crate) fn get_native(&self) -> PartialVMResult<&UnboxedNativeFunction> {
        self.native.as_deref().ok_or_else(|| {
            PartialVMError::new(StatusCode::MISSING_DEPENDENCY)
                .with_message(format!("Missing Native Function `{}`", self.name))
        })
    }
}

// A function instantiation.
#[derive(Clone, Debug)]
pub(crate) struct FunctionInstantiation {
    // index to `ModuleCache::functions` global table
    pub(crate) handle: FunctionHandle,
    pub(crate) instantiation: Vec<Type>,
}

#[derive(Clone, Debug)]
pub(crate) enum FunctionHandle {
    Local(Arc<Function>),
    Remote { module: ModuleId, name: Identifier },
}
