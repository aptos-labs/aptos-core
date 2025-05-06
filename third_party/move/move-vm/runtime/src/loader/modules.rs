// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{
        function::{Function, FunctionHandle, FunctionInstantiation},
        type_loader::intern_type,
    },
    native_functions::NativeFunctions,
};
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        Bytecode, CompiledModule, FieldDefinition, FieldHandleIndex, FieldInstantiationIndex,
        FunctionDefinitionIndex, SignatureIndex, StructDefinition, StructDefinitionIndex,
        StructFieldInformation, StructVariantHandleIndex, StructVariantInstantiationIndex,
        TableIndex, VariantFieldHandleIndex, VariantFieldInstantiationIndex, VariantIndex,
    },
};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::loaded_data::{
    runtime_types::{StructIdentifier, StructLayout, StructType, Type},
    struct_name_indexing::{StructNameIndex, StructNameIndexMap},
};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    ops::Deref,
    sync::Arc,
};

// A Module is very similar to a binary Module but data is "transformed" to a representation
// more appropriate to execution.
// When code executes indexes in instructions are resolved against those runtime structure
// so that any data needed for execution is immediately available
#[derive(Clone, Debug)]
pub struct Module {
    id: ModuleId,

    // size in bytes
    #[allow(dead_code)]
    pub(crate) size: usize,

    // primitive pools
    pub(crate) module: Arc<CompiledModule>,

    //
    // types as indexes into the Loader type list
    //
    pub(crate) structs: Vec<StructDef>,
    // materialized instantiations, whether partial or not
    pub(crate) struct_instantiations: Vec<StructInstantiation>,
    // same for struct variants
    pub(crate) struct_variant_infos: Vec<StructVariantInfo>,
    pub(crate) struct_variant_instantiation_infos: Vec<StructVariantInfo>,

    // functions as indexes into the Loader function list
    // That is effectively an indirection over the ref table:
    // the instruction carries an index into this table which contains the index into the
    // glabal table of functions. No instantiation of generic functions is saved into
    // the global table.
    pub(crate) function_refs: Vec<FunctionHandle>,
    pub(crate) function_defs: Vec<Arc<Function>>,
    // materialized instantiations, whether partial or not
    pub(crate) function_instantiations: Vec<FunctionInstantiation>,

    // fields as a pair of index, first to the type, second to the field position in that type
    pub(crate) field_handles: Vec<FieldHandle>,
    // materialized instantiations, whether partial or not
    pub(crate) field_instantiations: Vec<FieldInstantiation>,
    // Information about variant fields.
    pub(crate) variant_field_infos: Vec<VariantFieldInfo>,
    pub(crate) variant_field_instantiation_infos: Vec<VariantFieldInfo>,

    // function name to index into the Loader function list.
    // This allows a direct access from function name to `Function`
    pub(crate) function_map: HashMap<Identifier, usize>,
    // struct name to index into the module's type list
    // This allows a direct access from struct name to `Struct`
    pub(crate) struct_map: HashMap<Identifier, usize>,

    // a map of single-token signature indices to type.
    // Single-token signatures are usually indexed by the `SignatureIndex` in bytecode. For example,
    // `VecMutBorrow(SignatureIndex)`, the `SignatureIndex` maps to a single `SignatureToken`, and
    // hence, a single type.
    pub(crate) single_signature_token_map: BTreeMap<SignatureIndex, Type>,
}

#[derive(Clone, Debug)]
pub(crate) struct StructDef {
    pub(crate) field_count: u16,
    pub(crate) definition_struct_type: Arc<StructType>,
}

#[derive(Clone, Debug)]
pub(crate) struct StructInstantiation {
    pub(crate) field_count: u16,
    pub(crate) definition_struct_type: Arc<StructType>,
    pub(crate) instantiation: Vec<Type>,
}

#[derive(Clone, Debug)]
pub(crate) struct StructVariantInfo {
    pub(crate) field_count: u16,
    pub(crate) variant: VariantIndex,
    pub(crate) definition_struct_type: Arc<StructType>,
    pub(crate) instantiation: Vec<Type>,
}

// A field handle. The offset is the only used information when operating on a field
#[derive(Clone, Debug)]
pub(crate) struct FieldHandle {
    pub(crate) offset: usize,
    pub(crate) field_ty: Type,
    pub(crate) definition_struct_type: Arc<StructType>,
}

// A field instantiation. The offset is the only used information when operating on a field
#[derive(Clone, Debug)]
pub(crate) struct FieldInstantiation {
    pub(crate) offset: usize,
    pub(crate) uninstantiated_field_ty: Type,
    pub(crate) definition_struct_type: Arc<StructType>,
    pub(crate) instantiation: Vec<Type>,
}

// Information about to support both generic and non-generic variant fields.
#[derive(Clone, Debug)]
pub(crate) struct VariantFieldInfo {
    pub(crate) offset: usize,
    pub(crate) uninstantiated_field_ty: Type,
    pub(crate) variants: Vec<VariantIndex>,
    pub(crate) definition_struct_type: Arc<StructType>,
    pub(crate) instantiation: Vec<Type>,
}

impl Module {
    pub(crate) fn new(
        natives: &NativeFunctions,
        size: usize,
        module: Arc<CompiledModule>,
        struct_name_index_map: &StructNameIndexMap,
    ) -> PartialVMResult<Self> {
        let _timer = VM_TIMER.timer_with_label("Module::new");

        let id = module.self_id();

        let mut structs = vec![];
        let mut struct_instantiations = vec![];
        let mut struct_variant_infos = vec![];
        let mut struct_variant_instantiation_infos = vec![];
        let mut function_refs = vec![];
        let mut function_defs = vec![];
        let mut function_instantiations = vec![];
        let mut field_handles = vec![];
        let mut field_instantiations: Vec<FieldInstantiation> = vec![];
        let mut variant_field_infos = vec![];
        let mut variant_field_instantiation_infos = vec![];
        let mut function_map = HashMap::new();
        let mut struct_map = HashMap::new();
        let mut single_signature_token_map = BTreeMap::new();
        let mut signature_table = vec![];

        let mut create = || {
            let mut struct_idxs = vec![];
            let mut struct_names = vec![];
            // validate the correctness of struct handle references.
            for struct_handle in module.struct_handles() {
                let struct_name = module.identifier_at(struct_handle.name);
                let module_handle = module.module_handle_at(struct_handle.module);
                let module_id = module.module_id_for_handle(module_handle);

                let struct_name = StructIdentifier {
                    module: module_id,
                    name: struct_name.to_owned(),
                };
                struct_idxs.push(struct_name_index_map.struct_name_to_idx(&struct_name)?);
                struct_names.push(struct_name)
            }

            // Build signature table
            for signatures in module.signatures() {
                signature_table.push(
                    signatures
                        .0
                        .iter()
                        .map(|sig| {
                            intern_type(BinaryIndexedView::Module(&module), sig, &struct_idxs)
                        })
                        .collect::<PartialVMResult<Vec<_>>>()?,
                )
            }

            for (idx, struct_def) in module.struct_defs().iter().enumerate() {
                let definition_struct_type =
                    Arc::new(Self::make_struct_type(&module, struct_def, &struct_idxs)?);
                structs.push(StructDef {
                    field_count: definition_struct_type.field_count(None),
                    definition_struct_type,
                });
                let name =
                    module.identifier_at(module.struct_handle_at(struct_def.struct_handle).name);
                struct_map.insert(name.to_owned(), idx);
            }

            for struct_inst in module.struct_instantiations() {
                let def = struct_inst.def.0 as usize;
                let struct_def = &structs[def];
                struct_instantiations.push(StructInstantiation {
                    field_count: struct_def.definition_struct_type.field_count(None),
                    instantiation: signature_table[struct_inst.type_parameters.0 as usize].clone(),
                    definition_struct_type: struct_def.definition_struct_type.clone(),
                });
            }

            for struct_variant in module.struct_variant_handles() {
                let definition_struct_type = structs[struct_variant.struct_index.0 as usize]
                    .definition_struct_type
                    .clone();
                let variant = struct_variant.variant;
                struct_variant_infos.push(StructVariantInfo {
                    field_count: definition_struct_type.field_count(Some(variant)),
                    variant,
                    definition_struct_type,
                    instantiation: vec![],
                })
            }

            for struct_variant_inst in module.struct_variant_instantiations() {
                let variant = &struct_variant_infos[struct_variant_inst.handle.0 as usize];
                struct_variant_instantiation_infos.push(StructVariantInfo {
                    field_count: variant.field_count,
                    variant: variant.variant,
                    definition_struct_type: variant.definition_struct_type.clone(),
                    instantiation: signature_table[struct_variant_inst.type_parameters.0 as usize]
                        .clone(),
                })
            }

            for (idx, func) in module.function_defs().iter().enumerate() {
                let findex = FunctionDefinitionIndex(idx as TableIndex);
                let function = Function::new(
                    natives,
                    findex,
                    &module,
                    signature_table.as_slice(),
                    &struct_names,
                )?;

                function_map.insert(function.name.to_owned(), idx);
                function_defs.push(Arc::new(function));

                if let Some(code_unit) = &func.code {
                    for bc in &code_unit.code {
                        match bc {
                            Bytecode::CallClosure(si)
                            | Bytecode::VecPack(si, _)
                            | Bytecode::VecLen(si)
                            | Bytecode::VecImmBorrow(si)
                            | Bytecode::VecMutBorrow(si)
                            | Bytecode::VecPushBack(si)
                            | Bytecode::VecPopBack(si)
                            | Bytecode::VecUnpack(si, _)
                            | Bytecode::VecSwap(si) => {
                                if !single_signature_token_map.contains_key(si) {
                                    let ty = match module.signature_at(*si).0.first() {
                                        None => {
                                            return Err(PartialVMError::new(
                                                StatusCode::VERIFIER_INVARIANT_VIOLATION,
                                            )
                                            .with_message(
                                                "the type argument for vector-related bytecode \
                                                expects one and only one signature token"
                                                    .to_owned(),
                                            ));
                                        },
                                        Some(sig_token) => sig_token,
                                    };
                                    single_signature_token_map.insert(
                                        *si,
                                        intern_type(
                                            BinaryIndexedView::Module(&module),
                                            ty,
                                            &struct_idxs,
                                        )?,
                                    );
                                }
                            },
                            _ => {},
                        }
                    }
                }
            }

            for func_handle in module.function_handles() {
                let func_name = module.identifier_at(func_handle.name);
                let module_handle = module.module_handle_at(func_handle.module);
                let module_id = module.module_id_for_handle(module_handle);
                let func_handle = if module_id == id {
                    FunctionHandle::Local(
                        function_defs[*function_map.get(func_name).ok_or_else(|| {
                            PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE).with_message(
                                "Cannot find function in publishing module".to_string(),
                            )
                        })?]
                        .clone(),
                    )
                } else {
                    FunctionHandle::Remote {
                        module: module_id,
                        name: func_name.to_owned(),
                    }
                };
                function_refs.push(func_handle);
            }

            for func_inst in module.function_instantiations() {
                let handle = function_refs[func_inst.handle.0 as usize].clone();
                function_instantiations.push(FunctionInstantiation {
                    handle,
                    instantiation: signature_table[func_inst.type_parameters.0 as usize].clone(),
                });
            }

            for field_handle in module.field_handles() {
                let def_idx = field_handle.owner;
                let definition_struct_type =
                    structs[def_idx.0 as usize].definition_struct_type.clone();
                let offset = field_handle.field as usize;
                let ty = definition_struct_type.field_at(None, offset)?.1.clone();
                field_handles.push(FieldHandle {
                    offset,
                    field_ty: ty,
                    definition_struct_type,
                });
            }

            for field_inst in module.field_instantiations() {
                let fh_idx = field_inst.handle;
                let offset = field_handles[fh_idx.0 as usize].offset;
                let owner_struct_def = &structs[module.field_handle_at(fh_idx).owner.0 as usize];
                let uninstantiated_ty = owner_struct_def
                    .definition_struct_type
                    .field_at(None, offset)?
                    .1
                    .clone();
                field_instantiations.push(FieldInstantiation {
                    offset,
                    uninstantiated_field_ty: uninstantiated_ty,
                    instantiation: signature_table[field_inst.type_parameters.0 as usize].clone(),
                    definition_struct_type: owner_struct_def.definition_struct_type.clone(),
                });
            }

            for variant_handle in module.variant_field_handles() {
                let def_idx = variant_handle.struct_index;
                let definition_struct_type =
                    structs[def_idx.0 as usize].definition_struct_type.clone();
                let offset = variant_handle.field as usize;
                let variants = variant_handle.variants.clone();
                let ty = definition_struct_type
                    .field_at(Some(variants[0]), offset)?
                    .1
                    .clone();
                variant_field_infos.push(VariantFieldInfo {
                    offset,
                    variants,
                    definition_struct_type,
                    uninstantiated_field_ty: ty,
                    instantiation: vec![],
                });
            }

            for variant_inst in module.variant_field_instantiations() {
                let variant_info = &variant_field_infos[variant_inst.handle.0 as usize];
                let definition_struct_type = variant_info.definition_struct_type.clone();
                let variants = variant_info.variants.clone();
                let offset = variant_info.offset;
                let instantiation =
                    signature_table[variant_inst.type_parameters.0 as usize].clone();
                // We can select one representative variant for finding the field type, all
                // must have the same type as the verifier ensured.
                let uninstantiated_ty = definition_struct_type
                    .field_at(Some(variants[0]), offset)?
                    .1
                    .clone();
                variant_field_instantiation_infos.push(VariantFieldInfo {
                    offset,
                    uninstantiated_field_ty: uninstantiated_ty,
                    variants,
                    definition_struct_type,
                    instantiation,
                });
            }

            Ok(())
        };

        match create() {
            Ok(_) => Ok(Self {
                id,
                size,
                module,
                structs,
                struct_instantiations,
                struct_variant_infos,
                struct_variant_instantiation_infos,
                function_refs,
                function_defs,
                function_instantiations,
                field_handles,
                field_instantiations,
                variant_field_infos,
                variant_field_instantiation_infos,
                function_map,
                struct_map,
                single_signature_token_map,
            }),
            Err(err) => Err(err),
        }
    }
    
    /// Creates a new Module instance for testing purposes.
    /// This method creates a minimal Module with empty contents.
    #[cfg(any(test, feature = "testing"))]
    pub fn new_for_test(module_id: ModuleId) -> Self {
        use move_binary_format::file_format::empty_module;
        
        // Start with an empty module
        let mut empty_module = empty_module();
        
        // Update the module ID
        empty_module.identifiers[0] = module_id.name().to_owned();
        empty_module.address_identifiers[0] = *module_id.address();
        
        // Create necessary empty collections
        let module_arc = Arc::new(empty_module);
        
        Self {
            id: module_id,
            size: 0,
            module: module_arc,
            structs: vec![],
            struct_instantiations: vec![],
            struct_variant_infos: vec![],
            struct_variant_instantiation_infos: vec![],
            function_refs: vec![],
            function_defs: vec![],
            function_instantiations: vec![],
            field_handles: vec![],
            field_instantiations: vec![],
            variant_field_infos: vec![],
            variant_field_instantiation_infos: vec![],
            function_map: HashMap::new(),
            struct_map: HashMap::new(),
            single_signature_token_map: BTreeMap::new(),
        }
    }

    fn make_struct_type(
        module: &CompiledModule,
        struct_def: &StructDefinition,
        struct_name_table: &[StructNameIndex],
    ) -> PartialVMResult<StructType> {
        let struct_handle = module.struct_handle_at(struct_def.struct_handle);
        let abilities = struct_handle.abilities;
        let ty_params = struct_handle.type_parameters.clone();
        let layout = match &struct_def.field_information {
            StructFieldInformation::Native => unreachable!("native structs have been removed"),
            StructFieldInformation::Declared(fields) => {
                let fields: PartialVMResult<Vec<(Identifier, Type)>> = fields
                    .iter()
                    .map(|f| Self::make_field(module, f, struct_name_table))
                    .collect();
                StructLayout::Single(fields?)
            },
            StructFieldInformation::DeclaredVariants(variants) => {
                let variants: PartialVMResult<Vec<(Identifier, Vec<(Identifier, Type)>)>> =
                    variants
                        .iter()
                        .map(|v| {
                            let fields: PartialVMResult<Vec<(Identifier, Type)>> = v
                                .fields
                                .iter()
                                .map(|f| Self::make_field(module, f, struct_name_table))
                                .collect();
                            fields.map(|fields| (module.identifier_at(v.name).to_owned(), fields))
                        })
                        .collect();
                StructLayout::Variants(variants?)
            },
        };

        Ok(StructType {
            layout,
            phantom_ty_params_mask: struct_handle
                .type_parameters
                .iter()
                .map(|ty| ty.is_phantom)
                .collect(),
            abilities,
            ty_params,
            idx: struct_name_table[struct_def.struct_handle.0 as usize],
        })
    }

    fn make_field(
        module: &CompiledModule,
        field: &FieldDefinition,
        struct_name_table: &[StructNameIndex],
    ) -> PartialVMResult<(Identifier, Type)> {
        let ty = intern_type(
            BinaryIndexedView::Module(module),
            &field.signature.0,
            struct_name_table,
        )?;
        Ok((module.identifier_at(field.name).to_owned(), ty))
    }

    pub(crate) fn self_id(&self) -> &ModuleId {
        &self.id
    }

    pub(crate) fn struct_at(&self, idx: StructDefinitionIndex) -> &Arc<StructType> {
        &self.structs[idx.0 as usize].definition_struct_type
    }

    pub(crate) fn struct_instantiation_at(&self, idx: u16) -> &StructInstantiation {
        &self.struct_instantiations[idx as usize]
    }

    pub(crate) fn struct_variant_at(&self, idx: StructVariantHandleIndex) -> &StructVariantInfo {
        &self.struct_variant_infos[idx.0 as usize]
    }

    pub(crate) fn struct_variant_instantiation_at(
        &self,
        idx: StructVariantInstantiationIndex,
    ) -> &StructVariantInfo {
        &self.struct_variant_instantiation_infos[idx.0 as usize]
    }

    pub(crate) fn function_at(&self, idx: u16) -> &FunctionHandle {
        &self.function_refs[idx as usize]
    }

    pub(crate) fn function_instantiation_at(&self, idx: u16) -> &[Type] {
        &self.function_instantiations[idx as usize].instantiation
    }

    pub(crate) fn function_instantiation_handle_at(&self, idx: u16) -> &FunctionHandle {
        &self.function_instantiations[idx as usize].handle
    }

    pub(crate) fn field_count(&self, idx: u16) -> u16 {
        self.structs[idx as usize].field_count
    }

    pub(crate) fn field_instantiation_count(&self, idx: u16) -> u16 {
        self.struct_instantiations[idx as usize].field_count
    }

    pub(crate) fn field_offset(&self, idx: FieldHandleIndex) -> usize {
        self.field_handles[idx.0 as usize].offset
    }

    pub(crate) fn field_instantiation_offset(&self, idx: FieldInstantiationIndex) -> usize {
        self.field_instantiations[idx.0 as usize].offset
    }

    pub(crate) fn variant_field_info_at(&self, idx: VariantFieldHandleIndex) -> &VariantFieldInfo {
        &self.variant_field_infos[idx.0 as usize]
    }

    pub(crate) fn variant_field_instantiation_info_at(
        &self,
        idx: VariantFieldInstantiationIndex,
    ) -> &VariantFieldInfo {
        &self.variant_field_instantiation_infos[idx.0 as usize]
    }

    pub(crate) fn single_type_at(&self, idx: SignatureIndex) -> &Type {
        self.single_signature_token_map.get(&idx).unwrap()
    }

    pub(crate) fn get_function(&self, function_name: &IdentStr) -> VMResult<Arc<Function>> {
        Ok(self
            .function_map
            .get(function_name)
            .and_then(|idx| self.function_defs.get(*idx))
            .ok_or_else(|| {
                let module_id = self.self_id();
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                    .with_message(format!(
                        "Function {}::{}::{} does not exist",
                        module_id.address(),
                        module_id.name(),
                        function_name
                    ))
                    .finish(Location::Undefined)
            })?
            .clone())
    }

    pub(crate) fn get_struct(&self, struct_name: &IdentStr) -> VMResult<Arc<StructType>> {
        Ok(self
            .struct_map
            .get(struct_name)
            .and_then(|idx| self.structs.get(*idx))
            .ok_or_else(|| {
                let module_id = self.self_id();
                PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)
                    .with_message(format!(
                        "Struct {}::{}::{} does not exist",
                        module_id.address(),
                        module_id.name(),
                        struct_name
                    ))
                    .finish(Location::Undefined)
            })?
            .definition_struct_type
            .clone())
    }
}

impl Deref for Module {
    type Target = Arc<CompiledModule>;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}
