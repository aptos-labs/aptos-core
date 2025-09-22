// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    file_format::{
        AccessSpecifier, Bytecode, CodeOffset, CodeUnit, ConstantPoolIndex, FieldHandle,
        FieldHandleIndex, FieldInstantiation, FieldInstantiationIndex, FunctionDefinition,
        FunctionHandle, FunctionHandleIndex, FunctionInstantiation, FunctionInstantiationIndex,
        IdentifierIndex, LocalIndex, ModuleHandleIndex, Signature, SignatureIndex, SignatureToken,
        StructDefInstantiation, StructDefInstantiationIndex, StructDefinition,
        StructDefinitionIndex, StructFieldInformation, StructHandle, StructVariantHandle,
        StructVariantHandleIndex, StructVariantInstantiation, StructVariantInstantiationIndex,
        TableIndex, VariantFieldHandle, VariantFieldHandleIndex, VariantFieldInstantiation,
        VariantFieldInstantiationIndex, VariantIndex, Visibility,
    },
    internals::ModuleIndex,
    proptest_types::{
        prop_index_avoid,
        signature::{AbilitySetGen, SignatureGen, SignatureTokenGen},
        TableSize,
    },
};
use move_core_types::{ability::AbilitySet, u256::U256};
use proptest::{
    collection::{vec, SizeRange},
    option::of,
    prelude::*,
    sample::{select, Index as PropIndex},
};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    hash::Hash,
};

#[derive(Debug)]
struct HandleState<HandleT, IndexT> {
    handles: Vec<HandleT>,
    map: HashMap<HandleT, IndexT>,
}

impl<HandleT, IndexT> Default for HandleState<HandleT, IndexT> {
    fn default() -> Self {
        Self {
            handles: vec![],
            map: HashMap::new(),
        }
    }
}

impl<HandleT, IndexT> HandleState<HandleT, IndexT>
where
    HandleT: Hash + PartialEq + Eq + Clone,
    IndexT: Clone,
{
    fn add(&mut self, handle: HandleT, make_index: impl Fn(TableIndex) -> IndexT) -> IndexT {
        debug_assert!(self.handles.len() < TableSize::MAX as usize);
        if let Some(idx) = self.map.get(&handle) {
            return idx.clone();
        }
        let idx = make_index(self.handles.len() as TableIndex);
        self.handles.push(handle.clone());
        self.map.insert(handle, idx.clone());
        idx
    }

    fn handles(self) -> Vec<HandleT> {
        self.handles
    }
}

type SignatureState = HandleState<Signature, SignatureIndex>;
type FieldHandleState = HandleState<FieldHandle, FieldHandleIndex>;
type VariantFieldHandleState = HandleState<VariantFieldHandle, VariantFieldHandleIndex>;
type StructVariantHandleState = HandleState<StructVariantHandle, StructVariantHandleIndex>;

impl SignatureState {
    fn from(sigs: Vec<Signature>) -> Self {
        let mut state = Self::default();
        for sig in sigs {
            state.add(sig, SignatureIndex);
        }
        state
    }
}

#[derive(Debug)]
#[allow(unused)]
struct InstantiationState<T>
where
    T: Eq + Clone + Hash,
{
    instantiations: Vec<T>,
    instantiation_map: HashMap<T, TableIndex>,
}

impl<T> InstantiationState<T>
where
    T: Eq + Clone + Hash,
{
    fn new() -> Self {
        InstantiationState {
            instantiations: vec![],
            instantiation_map: HashMap::new(),
        }
    }

    #[allow(unused)]
    pub fn instantiations(self) -> Vec<T> {
        self.instantiations
    }

    #[allow(unused)]
    fn add_instantiation(&mut self, inst: T) -> TableIndex {
        debug_assert!(self.instantiations.len() < TableSize::MAX as usize);
        if let Some(idx) = self.instantiation_map.get(&inst) {
            return *idx;
        }
        let idx = self.instantiations.len() as TableIndex;
        self.instantiations.push(inst.clone());
        self.instantiation_map.insert(inst, idx);
        idx
    }
}

/// Represents state required to materialize final data structures for function handles.
#[derive(Debug)]
pub struct FnHandleMaterializeState<'a> {
    self_module_handle_idx: ModuleHandleIndex,
    module_handles_len: usize,
    identifiers_len: usize,
    struct_handles: &'a [StructHandle],
    signatures: SignatureState,
    function_handles: HashSet<(ModuleHandleIndex, IdentifierIndex)>,
}

impl<'a> FnHandleMaterializeState<'a> {
    pub fn new(
        self_module_handle_idx: ModuleHandleIndex,
        module_handles_len: usize,
        identifiers_len: usize,
        struct_handles: &'a [StructHandle],
        signatures: Vec<Signature>,
    ) -> Self {
        Self {
            self_module_handle_idx,
            module_handles_len,
            identifiers_len,
            struct_handles,
            signatures: SignatureState::from(signatures),
            function_handles: HashSet::new(),
        }
    }

    pub fn signatures(self) -> Vec<Signature> {
        self.signatures.handles()
    }

    fn add_signature(&mut self, sig: Signature) -> SignatureIndex {
        self.signatures.add(sig, SignatureIndex)
    }
}

#[derive(Clone, Debug)]
pub struct FunctionHandleGen {
    module: PropIndex,
    name: PropIndex,
    parameters: SignatureGen,
    return_: SignatureGen,
    type_parameters: Vec<AbilitySetGen>,
    access_specifiers: Option<Vec<AccessSpecifier>>,
}

impl FunctionHandleGen {
    pub fn strategy(
        param_count: impl Into<SizeRange>,
        return_count: impl Into<SizeRange>,
        type_parameter_count: impl Into<SizeRange>,
        access_specifiers_count: impl Into<SizeRange>,
    ) -> impl Strategy<Value = Self> {
        let return_count = return_count.into();
        let param_count = param_count.into();
        (
            any::<PropIndex>(),
            any::<PropIndex>(),
            SignatureGen::strategy(param_count),
            SignatureGen::strategy(return_count),
            vec(AbilitySetGen::strategy(), type_parameter_count),
            of(vec(any::<AccessSpecifier>(), access_specifiers_count)),
        )
            .prop_map(
                |(module, name, parameters, return_, type_parameters, access_specifiers)| Self {
                    module,
                    name,
                    parameters,
                    return_,
                    type_parameters,
                    access_specifiers,
                },
            )
    }

    pub fn materialize(self, state: &mut FnHandleMaterializeState) -> Option<FunctionHandle> {
        let idx = prop_index_avoid(
            self.module,
            state.self_module_handle_idx.into_index(),
            state.module_handles_len,
        );
        let mod_idx = ModuleHandleIndex(idx as TableIndex);
        let iden_idx = IdentifierIndex(self.name.index(state.identifiers_len) as TableIndex);
        if state.function_handles.contains(&(mod_idx, iden_idx)) {
            return None;
        }
        state.function_handles.insert((mod_idx, iden_idx));
        let parameters = self.parameters.materialize(state.struct_handles);
        let params_idx = state.add_signature(parameters);
        let return_ = self.return_.materialize(state.struct_handles);
        let return_idx = state.add_signature(return_);
        let type_parameters = self
            .type_parameters
            .into_iter()
            .map(|abilities| abilities.materialize())
            .collect();
        Some(FunctionHandle {
            module: mod_idx,
            name: iden_idx,
            parameters: params_idx,
            return_: return_idx,
            type_parameters,
            access_specifiers: self.access_specifiers,
            attributes: vec![],
        })
    }
}

/// Represents state required to materialize final data structures for function definitions.
#[derive(Debug)]
pub struct FnDefnMaterializeState<'a> {
    self_module_handle_idx: ModuleHandleIndex,
    identifiers_len: usize,
    constant_pool_len: usize,
    struct_handles: &'a [StructHandle],
    struct_defs: &'a [StructDefinition],
    signatures: SignatureState,
    function_handles: Vec<FunctionHandle>,
    def_function_handles: HashSet<(ModuleHandleIndex, IdentifierIndex)>,
    field_handles: FieldHandleState,
    variant_field_handles: VariantFieldHandleState,
    type_instantiations: InstantiationState<StructDefInstantiation>,
    function_instantiations: InstantiationState<FunctionInstantiation>,
    field_instantiations: InstantiationState<FieldInstantiation>,
    variant_field_instantiations: InstantiationState<VariantFieldInstantiation>,
    struct_variant_handles: StructVariantHandleState,
    struct_variant_instantiations: InstantiationState<StructVariantInstantiation>,
}

impl<'a> FnDefnMaterializeState<'a> {
    pub fn new(
        self_module_handle_idx: ModuleHandleIndex,
        identifiers_len: usize,
        constant_pool_len: usize,
        struct_handles: &'a [StructHandle],
        struct_defs: &'a [StructDefinition],
        signatures: Vec<Signature>,
        function_handles: Vec<FunctionHandle>,
    ) -> Self {
        Self {
            self_module_handle_idx,
            identifiers_len,
            constant_pool_len,
            struct_handles,
            struct_defs,
            signatures: SignatureState::from(signatures),
            function_handles,
            def_function_handles: HashSet::new(),
            field_handles: FieldHandleState::default(),
            variant_field_handles: VariantFieldHandleState::default(),
            type_instantiations: InstantiationState::new(),
            function_instantiations: InstantiationState::new(),
            field_instantiations: InstantiationState::new(),
            variant_field_instantiations: InstantiationState::new(),
            struct_variant_handles: StructVariantHandleState::default(),
            struct_variant_instantiations: InstantiationState::new(),
        }
    }

    pub fn return_tables(
        self,
    ) -> (
        Vec<Signature>,
        Vec<FunctionHandle>,
        Vec<FieldHandle>,
        Vec<StructDefInstantiation>,
        Vec<FunctionInstantiation>,
        Vec<FieldInstantiation>,
        Vec<VariantFieldHandle>,
        Vec<VariantFieldInstantiation>,
        Vec<StructVariantHandle>,
        Vec<StructVariantInstantiation>,
    ) {
        (
            self.signatures.handles(),
            self.function_handles,
            self.field_handles.handles(),
            self.type_instantiations.instantiations(),
            self.function_instantiations.instantiations(),
            self.field_instantiations.instantiations(),
            self.variant_field_handles.handles(),
            self.variant_field_instantiations.instantiations(),
            self.struct_variant_handles.handles(),
            self.struct_variant_instantiations.instantiations(),
        )
    }

    fn add_signature(&mut self, sig: Signature) -> SignatureIndex {
        self.signatures.add(sig, SignatureIndex)
    }

    fn add_function_handle(&mut self, handle: FunctionHandle) -> FunctionHandleIndex {
        debug_assert!(self.function_handles.len() < TableSize::MAX as usize);
        self.function_handles.push(handle);
        FunctionHandleIndex((self.function_handles.len() - 1) as TableIndex)
    }

    fn get_signature_from_type_params(
        &mut self,
        abilities: impl IntoIterator<Item = AbilitySet>,
    ) -> Signature {
        let mut type_params = vec![];
        for abs in abilities {
            assert!(!abs.has_key());
            match (abs.has_copy(), abs.has_drop(), abs.has_store()) {
                (false, true, false) => type_params.push(SignatureToken::Signer),
                _ => type_params.push(SignatureToken::U64),
            }
        }
        Signature(type_params)
    }

    fn add_signature_from_type_params(
        &mut self,
        abilities: impl IntoIterator<Item = AbilitySet>,
    ) -> SignatureIndex {
        let sig = self.get_signature_from_type_params(abilities);
        self.signatures.add(sig, SignatureIndex)
    }

    fn get_function_instantiation(&mut self, fh_idx: usize) -> FunctionInstantiationIndex {
        let abilities = self.function_handles[fh_idx].type_parameters.clone();
        let sig_idx = self.add_signature_from_type_params(abilities.iter().copied());
        let fi = FunctionInstantiation {
            handle: FunctionHandleIndex(fh_idx as TableIndex),
            type_parameters: sig_idx,
        };
        FunctionInstantiationIndex(self.function_instantiations.add_instantiation(fi))
    }

    fn get_type_instantiation(&mut self, sd_idx: usize) -> StructDefInstantiationIndex {
        let sig_idx = self.get_struct_instantiation_sig(sd_idx);
        let si = StructDefInstantiation {
            def: StructDefinitionIndex(sd_idx as TableIndex),
            type_parameters: sig_idx,
        };
        StructDefInstantiationIndex(self.type_instantiations.add_instantiation(si))
    }

    fn get_struct_instantiation_sig(&mut self, sd_idx: usize) -> SignatureIndex {
        let sd = &self.struct_defs[sd_idx];
        let struct_handle = &self.struct_handles[sd.struct_handle.0 as usize];
        self.add_signature_from_type_params(struct_handle.type_param_constraints())
    }
}

#[derive(Clone, Debug)]
pub struct FunctionDefinitionGen {
    name: PropIndex,
    parameters: SignatureGen,
    return_: SignatureGen,
    visibility: Visibility,
    is_entry: bool,
    acquires: Vec<PropIndex>,
    code: CodeUnitGen,
}

impl FunctionDefinitionGen {
    pub fn strategy(
        return_count: impl Into<SizeRange>,
        arg_count: impl Into<SizeRange>,
        _type_parameter_count: impl Into<SizeRange>,
        acquires_count: impl Into<SizeRange>,
        code_len: impl Into<SizeRange>,
    ) -> impl Strategy<Value = Self> {
        let return_count = return_count.into();
        let arg_count = arg_count.into();
        (
            any::<PropIndex>(),
            SignatureGen::strategy(arg_count.clone()),
            SignatureGen::strategy(return_count),
            any::<Visibility>(),
            any::<bool>(),
            vec(any::<PropIndex>(), acquires_count.into()),
            CodeUnitGen::strategy(arg_count, code_len),
        )
            .prop_map(
                |(name, parameters, return_, visibility, is_entry, acquires, code)| Self {
                    name,
                    parameters,
                    return_,
                    visibility,
                    is_entry,
                    acquires,
                    code,
                },
            )
    }

    pub fn materialize(self, state: &mut FnDefnMaterializeState) -> Option<FunctionDefinition> {
        // This precondition should never fail because the table size cannot be greater
        // than TableSize::max_value()
        let iden_idx = IdentifierIndex(self.name.index(state.identifiers_len) as TableIndex);
        if state
            .def_function_handles
            .contains(&(state.self_module_handle_idx, iden_idx))
        {
            return None;
        }
        state
            .def_function_handles
            .insert((state.self_module_handle_idx, iden_idx));

        let parameters = self.parameters.materialize(state.struct_handles);
        let params_idx = state.add_signature(parameters);
        let return_ = self.return_.materialize(state.struct_handles);
        let return_idx = state.add_signature(return_);
        let handle = FunctionHandle {
            module: state.self_module_handle_idx,
            name: iden_idx,
            parameters: params_idx,
            return_: return_idx,
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        };
        let function_handle = state.add_function_handle(handle);
        let mut acquires_set = BTreeSet::new();
        for acquire in self.acquires {
            acquires_set.insert(StructDefinitionIndex(
                acquire.index(state.struct_defs.len()) as TableIndex,
            ));
        }
        let acquires_global_resources = acquires_set.into_iter().collect();
        // TODO: consider generating native functions?
        Some(FunctionDefinition {
            function: function_handle,
            visibility: self.visibility,
            is_entry: self.is_entry,
            acquires_global_resources,
            code: Some(self.code.materialize(state)),
        })
    }
}

#[derive(Clone, Debug)]
struct CodeUnitGen {
    locals_signature: Vec<SignatureTokenGen>,
    code: Vec<BytecodeGen>,
}

impl CodeUnitGen {
    fn strategy(
        arg_count: impl Into<SizeRange>,
        code_len: impl Into<SizeRange>,
    ) -> impl Strategy<Value = Self> {
        (
            vec(SignatureTokenGen::strategy(), arg_count),
            vec(BytecodeGen::garbage_strategy(), code_len),
        )
            .prop_map(|(locals_signature, code)| Self {
                locals_signature,
                code,
            })
    }

    fn materialize(self, state: &mut FnDefnMaterializeState) -> CodeUnit {
        let locals_signature = Signature(
            self.locals_signature
                .into_iter()
                .map(|sig| sig.materialize(state.struct_handles))
                .collect(),
        );

        let mut code = vec![];
        for bytecode_gen in self.code {
            if let Some(bytecode) = bytecode_gen.materialize(state, code.len(), &locals_signature) {
                code.push(bytecode)
            }
        }

        CodeUnit {
            locals: state.add_signature(locals_signature),
            code,
        }
    }
}

#[derive(Clone, Debug)]
enum BytecodeGen {
    // "Simple" means this doesn't refer to any other indexes.
    Simple(Bytecode),
    // All of these refer to other indexes.
    LdConst(PropIndex),

    MutBorrowField((PropIndex, PropIndex)),
    ImmBorrowField((PropIndex, PropIndex)),
    MutBorrowVariantField((PropIndex, PropIndex, PropIndex)),
    ImmBorrowVariantField((PropIndex, PropIndex, PropIndex)),

    Call(PropIndex),

    Pack(PropIndex),
    Unpack(PropIndex),
    PackVariant((PropIndex, PropIndex)),
    UnpackVariant((PropIndex, PropIndex)),

    Exists(PropIndex),
    MutBorrowGlobal(PropIndex),
    ImmBorrowGlobal(PropIndex),
    MoveFrom(PropIndex),
    MoveTo(PropIndex),
    BrTrue(PropIndex),
    BrFalse(PropIndex),
    Branch(PropIndex),
    CopyLoc(PropIndex),
    MoveLoc(PropIndex),
    StLoc(PropIndex),
    MutBorrowLoc(PropIndex),
    ImmBorrowLoc(PropIndex),

    VecPack((PropIndex, u64)),
    VecLen(PropIndex),
    VecImmBorrow(PropIndex),
    VecMutBorrow(PropIndex),
    VecPushBack(PropIndex),
    VecPopBack(PropIndex),
    VecUnpack((PropIndex, u64)),
    VecSwap(PropIndex),
}

impl BytecodeGen {
    // This just generates nonsensical bytecodes. This will be cleaned up later as the generation
    // model is refined.
    fn garbage_strategy() -> impl Strategy<Value = Self> {
        use BytecodeGen::*;

        prop_oneof![
            Self::simple_bytecode_strategy().prop_map(Simple),
            any::<PropIndex>().prop_map(LdConst),
            (any::<PropIndex>(), any::<PropIndex>()).prop_map(ImmBorrowField),
            (any::<PropIndex>(), any::<PropIndex>()).prop_map(MutBorrowField),
            (any::<PropIndex>(), any::<PropIndex>(), any::<PropIndex>())
                .prop_map(ImmBorrowVariantField),
            (any::<PropIndex>(), any::<PropIndex>(), any::<PropIndex>())
                .prop_map(MutBorrowVariantField),
            any::<PropIndex>().prop_map(Call),
            any::<PropIndex>().prop_map(Pack),
            any::<PropIndex>().prop_map(Unpack),
            (any::<PropIndex>(), any::<PropIndex>()).prop_map(PackVariant),
            (any::<PropIndex>(), any::<PropIndex>()).prop_map(UnpackVariant),
            any::<PropIndex>().prop_map(Exists),
            any::<PropIndex>().prop_map(ImmBorrowGlobal),
            any::<PropIndex>().prop_map(MutBorrowGlobal),
            any::<PropIndex>().prop_map(MoveFrom),
            any::<PropIndex>().prop_map(MoveTo),
            any::<PropIndex>().prop_map(BrTrue),
            any::<PropIndex>().prop_map(BrFalse),
            any::<PropIndex>().prop_map(Branch),
            any::<PropIndex>().prop_map(CopyLoc),
            any::<PropIndex>().prop_map(MoveLoc),
            any::<PropIndex>().prop_map(StLoc),
            any::<PropIndex>().prop_map(MutBorrowLoc),
            any::<PropIndex>().prop_map(ImmBorrowLoc),
            (any::<PropIndex>(), any::<u64>()).prop_map(VecPack),
            any::<PropIndex>().prop_map(VecLen),
            any::<PropIndex>().prop_map(VecImmBorrow),
            any::<PropIndex>().prop_map(VecMutBorrow),
            any::<PropIndex>().prop_map(VecPushBack),
            any::<PropIndex>().prop_map(VecPopBack),
            (any::<PropIndex>(), any::<u64>()).prop_map(VecUnpack),
            any::<PropIndex>().prop_map(VecSwap),
        ]
    }

    fn materialize(
        self,
        state: &mut FnDefnMaterializeState,
        code_len: usize,
        locals_signature: &Signature,
    ) -> Option<Bytecode> {
        let bytecode = match self {
            BytecodeGen::Simple(bytecode) => bytecode,
            BytecodeGen::LdConst(idx) => {
                if state.constant_pool_len == 0 {
                    return None;
                }
                Bytecode::LdConst(ConstantPoolIndex(
                    idx.index(state.constant_pool_len) as TableIndex
                ))
            },
            BytecodeGen::MutBorrowField((def, field)) => {
                Self::materialize_borrow(state, def, false, None, field)?
            },
            BytecodeGen::ImmBorrowField((def, field)) => {
                Self::materialize_borrow(state, def, true, None, field)?
            },
            BytecodeGen::MutBorrowVariantField((def, variant, field)) => {
                Self::materialize_borrow(state, def, false, Some(variant), field)?
            },
            BytecodeGen::ImmBorrowVariantField((def, variant, field)) => {
                Self::materialize_borrow(state, def, true, Some(variant), field)?
            },
            BytecodeGen::Call(idx) => {
                let func_handles_len = state.function_handles.len();
                let fh_idx = idx.index(func_handles_len);

                if state.function_handles[fh_idx].type_parameters.is_empty() {
                    Bytecode::Call(FunctionHandleIndex(fh_idx as TableIndex))
                } else {
                    Bytecode::CallGeneric(state.get_function_instantiation(fh_idx))
                }
            },
            BytecodeGen::Pack(def) => Self::materialize_pack_unpack(state, true, def, None)?,
            BytecodeGen::Unpack(def) => Self::materialize_pack_unpack(state, false, def, None)?,
            BytecodeGen::PackVariant((def, variant)) => {
                Self::materialize_pack_unpack(state, true, def, Some(variant))?
            },
            BytecodeGen::UnpackVariant((def, variant)) => {
                Self::materialize_pack_unpack(state, false, def, Some(variant))?
            },

            BytecodeGen::Exists(idx) => {
                let struct_defs_len = state.struct_defs.len();
                let sd_idx = idx.index(struct_defs_len);

                let sd = &state.struct_defs[sd_idx];
                if state.struct_handles[sd.struct_handle.0 as usize]
                    .type_parameters
                    .is_empty()
                {
                    Bytecode::Exists(StructDefinitionIndex(sd_idx as TableIndex))
                } else {
                    Bytecode::ExistsGeneric(state.get_type_instantiation(sd_idx))
                }
            },
            BytecodeGen::ImmBorrowGlobal(idx) => {
                let struct_defs_len = state.struct_defs.len();
                let sd_idx = idx.index(struct_defs_len);

                let sd = &state.struct_defs[sd_idx];
                if state.struct_handles[sd.struct_handle.0 as usize]
                    .type_parameters
                    .is_empty()
                {
                    Bytecode::ImmBorrowGlobal(StructDefinitionIndex(sd_idx as TableIndex))
                } else {
                    Bytecode::ImmBorrowGlobalGeneric(state.get_type_instantiation(sd_idx))
                }
            },
            BytecodeGen::MutBorrowGlobal(idx) => {
                let struct_defs_len = state.struct_defs.len();
                let sd_idx = idx.index(struct_defs_len);

                let sd = &state.struct_defs[sd_idx];
                if state.struct_handles[sd.struct_handle.0 as usize]
                    .type_parameters
                    .is_empty()
                {
                    Bytecode::MutBorrowGlobal(StructDefinitionIndex(sd_idx as TableIndex))
                } else {
                    Bytecode::MutBorrowGlobalGeneric(state.get_type_instantiation(sd_idx))
                }
            },
            BytecodeGen::MoveFrom(idx) => {
                let struct_defs_len = state.struct_defs.len();
                let sd_idx = idx.index(struct_defs_len);

                let sd = &state.struct_defs[sd_idx];
                if state.struct_handles[sd.struct_handle.0 as usize]
                    .type_parameters
                    .is_empty()
                {
                    Bytecode::MoveFrom(StructDefinitionIndex(sd_idx as TableIndex))
                } else {
                    Bytecode::MoveFromGeneric(state.get_type_instantiation(sd_idx))
                }
            },
            BytecodeGen::MoveTo(idx) => {
                let struct_defs_len = state.struct_defs.len();
                let sd_idx = idx.index(struct_defs_len);

                let sd = &state.struct_defs[sd_idx];
                if state.struct_handles[sd.struct_handle.0 as usize]
                    .type_parameters
                    .is_empty()
                {
                    Bytecode::MoveTo(StructDefinitionIndex(sd_idx as TableIndex))
                } else {
                    Bytecode::MoveToGeneric(state.get_type_instantiation(sd_idx))
                }
            },
            BytecodeGen::BrTrue(idx) => {
                if code_len == 0 {
                    return None;
                }
                Bytecode::BrTrue(idx.index(code_len) as CodeOffset)
            },
            BytecodeGen::BrFalse(idx) => {
                if code_len == 0 {
                    return None;
                }
                Bytecode::BrFalse(idx.index(code_len) as CodeOffset)
            },
            BytecodeGen::Branch(idx) => {
                if code_len == 0 {
                    return None;
                }
                Bytecode::Branch(idx.index(code_len) as CodeOffset)
            },
            BytecodeGen::CopyLoc(idx) => {
                if locals_signature.is_empty() {
                    return None;
                }
                Bytecode::CopyLoc(idx.index(locals_signature.len()) as LocalIndex)
            },
            BytecodeGen::MoveLoc(idx) => {
                if locals_signature.is_empty() {
                    return None;
                }
                Bytecode::MoveLoc(idx.index(locals_signature.len()) as LocalIndex)
            },
            BytecodeGen::StLoc(idx) => {
                if locals_signature.is_empty() {
                    return None;
                }
                Bytecode::StLoc(idx.index(locals_signature.len()) as LocalIndex)
            },
            BytecodeGen::MutBorrowLoc(idx) => {
                if locals_signature.is_empty() {
                    return None;
                }
                Bytecode::MutBorrowLoc(idx.index(locals_signature.len()) as LocalIndex)
            },
            BytecodeGen::ImmBorrowLoc(idx) => {
                if locals_signature.is_empty() {
                    return None;
                }
                Bytecode::ImmBorrowLoc(idx.index(locals_signature.len()) as LocalIndex)
            },
            BytecodeGen::VecPack((idx, num)) => {
                if num > u16::MAX as u64 {
                    return None;
                }
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecPack(SignatureIndex(sig_idx as TableIndex), num)
            },
            BytecodeGen::VecLen(idx) => {
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecLen(SignatureIndex(sig_idx as TableIndex))
            },
            BytecodeGen::VecImmBorrow(idx) => {
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecImmBorrow(SignatureIndex(sig_idx as TableIndex))
            },
            BytecodeGen::VecMutBorrow(idx) => {
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecMutBorrow(SignatureIndex(sig_idx as TableIndex))
            },
            BytecodeGen::VecPushBack(idx) => {
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecPushBack(SignatureIndex(sig_idx as TableIndex))
            },
            BytecodeGen::VecPopBack(idx) => {
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecPopBack(SignatureIndex(sig_idx as TableIndex))
            },
            BytecodeGen::VecUnpack((idx, num)) => {
                if num > u16::MAX as u64 {
                    return None;
                }
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecUnpack(SignatureIndex(sig_idx as TableIndex), num)
            },
            BytecodeGen::VecSwap(idx) => {
                let sigs_len = state.signatures.handles.len();
                if sigs_len == 0 {
                    return None;
                }
                let sig_idx = idx.index(sigs_len);
                let sig = &state.signatures.handles[sig_idx];
                if !BytecodeGen::is_valid_vector_element_sig(sig) {
                    return None;
                }
                Bytecode::VecSwap(SignatureIndex(sig_idx as TableIndex))
            },
        };

        Some(bytecode)
    }

    fn materialize_pack_unpack(
        state: &mut FnDefnMaterializeState,
        is_pack: bool,
        def: PropIndex,
        variant: Option<PropIndex>,
    ) -> Option<Bytecode> {
        let struct_defs_len = state.struct_defs.len();
        let struct_idx = def.index(struct_defs_len);
        let struct_def_idx = StructDefinitionIndex(struct_idx as TableIndex);
        let struct_def = &state.struct_defs[struct_idx];
        let struct_handle = &state.struct_handles[struct_def.struct_handle.0 as usize];
        let make_opc = |handle: StructDefinitionIndex| {
            if is_pack {
                Bytecode::Pack(handle)
            } else {
                Bytecode::Unpack(handle)
            }
        };
        let make_gen_opc = |handle: StructDefInstantiationIndex| {
            if is_pack {
                Bytecode::PackGeneric(handle)
            } else {
                Bytecode::UnpackGeneric(handle)
            }
        };
        let make_variant_opc = |handle: StructVariantHandleIndex| {
            if is_pack {
                Bytecode::PackVariant(handle)
            } else {
                Bytecode::UnpackVariant(handle)
            }
        };
        let make_gen_variant_opc = |handle: StructVariantInstantiationIndex| {
            if is_pack {
                Bytecode::PackVariantGeneric(handle)
            } else {
                Bytecode::UnpackVariantGeneric(handle)
            }
        };
        match (&struct_def.field_information, variant) {
            (StructFieldInformation::Declared(..), None) => {
                if struct_handle.type_parameters.is_empty() {
                    Some(make_opc(struct_def_idx))
                } else {
                    let sig_idx = state
                        .add_signature_from_type_params(struct_handle.type_param_constraints());
                    let si_idx =
                        state
                            .type_instantiations
                            .add_instantiation(StructDefInstantiation {
                                def: struct_def_idx,
                                type_parameters: sig_idx,
                            });
                    Some(make_gen_opc(StructDefInstantiationIndex(
                        si_idx as TableIndex,
                    )))
                }
            },
            (StructFieldInformation::DeclaredVariants(..), Some(var_index)) => {
                let variant =
                    var_index.index(struct_def.field_information.variant_count()) as VariantIndex;
                // Create a new variant handle. Currently, this is not internalized as some
                // other handles.
                let sv_idx = state.struct_variant_handles.add(
                    StructVariantHandle {
                        struct_index: struct_def_idx,
                        variant,
                    },
                    StructVariantHandleIndex,
                );
                if struct_handle.type_parameters.is_empty() {
                    Some(make_variant_opc(sv_idx))
                } else {
                    let sig_idx = state
                        .add_signature_from_type_params(struct_handle.type_param_constraints());
                    let si_idx = StructVariantInstantiationIndex(
                        state.struct_variant_instantiations.add_instantiation(
                            StructVariantInstantiation {
                                handle: sv_idx,
                                type_parameters: sig_idx,
                            },
                        ) as TableIndex,
                    );
                    Some(make_gen_variant_opc(si_idx))
                }
            },
            _ => {
                // All other cases pack/unpack not available
                None
            },
        }
    }

    fn materialize_borrow(
        state: &mut FnDefnMaterializeState,
        def: PropIndex,
        is_imm_borrow: bool,
        variant: Option<PropIndex>,
        field: PropIndex,
    ) -> Option<Bytecode> {
        let sd_idx = def.index(state.struct_defs.len());
        let struct_def = &state.struct_defs[sd_idx];
        let struct_handle = &state.struct_handles[struct_def.struct_handle.0 as usize];
        let make_opc = |handle: FieldHandleIndex| {
            if is_imm_borrow {
                Bytecode::ImmBorrowField(handle)
            } else {
                Bytecode::MutBorrowField(handle)
            }
        };
        let make_gen_opc = |handle: FieldInstantiationIndex| {
            if is_imm_borrow {
                Bytecode::ImmBorrowFieldGeneric(handle)
            } else {
                Bytecode::MutBorrowFieldGeneric(handle)
            }
        };
        let make_variant_opc = |handle: VariantFieldHandleIndex| {
            if is_imm_borrow {
                Bytecode::ImmBorrowVariantField(handle)
            } else {
                Bytecode::MutBorrowVariantField(handle)
            }
        };
        let make_gen_variant_opc = |handle: VariantFieldInstantiationIndex| {
            if is_imm_borrow {
                Bytecode::ImmBorrowVariantFieldGeneric(handle)
            } else {
                Bytecode::MutBorrowVariantFieldGeneric(handle)
            }
        };
        match (&struct_def.field_information, variant) {
            (StructFieldInformation::Declared(..), None) => {
                let field_count = struct_def.field_information.field_count(None);
                if field_count == 0 {
                    return None;
                }
                let fh_idx = state.field_handles.add(
                    FieldHandle {
                        owner: StructDefinitionIndex(sd_idx as TableIndex),
                        field: field.index(field_count) as TableIndex,
                    },
                    FieldHandleIndex,
                );
                if struct_handle.type_parameters.is_empty() {
                    Some(make_opc(fh_idx))
                } else {
                    let sig_idx = state
                        .add_signature_from_type_params(struct_handle.type_param_constraints());
                    let fi_idx = state
                        .field_instantiations
                        .add_instantiation(FieldInstantiation {
                            handle: fh_idx,
                            type_parameters: sig_idx,
                        });
                    Some(make_gen_opc(FieldInstantiationIndex(fi_idx as TableIndex)))
                }
            },
            (StructFieldInformation::DeclaredVariants(..), Some(var_index)) => {
                let variant =
                    var_index.index(struct_def.field_information.variant_count()) as VariantIndex;
                let field_count = struct_def.field_information.field_count(Some(variant));
                if field_count == 0 {
                    return None;
                }
                let fh_idx = state.variant_field_handles.add(
                    VariantFieldHandle {
                        struct_index: StructDefinitionIndex(sd_idx as TableIndex),
                        variants: vec![variant],
                        field: field.index(field_count) as TableIndex,
                    },
                    VariantFieldHandleIndex,
                );
                if struct_handle.type_parameters.is_empty() {
                    Some(make_variant_opc(fh_idx))
                } else {
                    let sig_idx = state
                        .add_signature_from_type_params(struct_handle.type_param_constraints());
                    let fi_idx = state.variant_field_instantiations.add_instantiation(
                        VariantFieldInstantiation {
                            handle: fh_idx,
                            type_parameters: sig_idx,
                        },
                    );
                    Some(make_gen_variant_opc(VariantFieldInstantiationIndex(
                        fi_idx as TableIndex,
                    )))
                }
            },
            _ => {
                // All other cases borrow not available
                None
            },
        }
    }

    /// Checks if the given type is well defined in the given context.
    /// No references are permitted.
    fn check_signature_token(token: &SignatureToken) -> bool {
        use SignatureToken::*;
        match token {
            U8 | U16 | U32 | U64 | U128 | U256 | I64 | I128 | Bool | Address | Signer | Struct(_)
            | TypeParameter(_) => true,
            Vector(element_token) => BytecodeGen::check_signature_token(element_token),
            StructInstantiation(_, type_arguments) => type_arguments
                .iter()
                .all(BytecodeGen::check_signature_token),
            Function(params, results, _) => params
                .iter()
                .chain(results)
                .all(BytecodeGen::check_signature_token),
            Reference(_) | MutableReference(_) => false,
        }
    }

    fn is_valid_vector_element_sig(sig: &Signature) -> bool {
        if sig.len() != 1 {
            return false;
        }
        BytecodeGen::check_signature_token(&sig.0[0])
    }

    fn simple_bytecode_strategy() -> impl Strategy<Value = Bytecode> {
        prop_oneof![
        // The numbers are relative weights, somewhat arbitrarily picked.
        9 => Self::just_bytecode_strategy(),
        1 => any::<u64>().prop_map(Bytecode::LdU64),
        1 => any::<u8>().prop_map(Bytecode::LdU8),
        1 => any::<u128>().prop_map(Bytecode::LdU128),
        1 => any::<u16>().prop_map(Bytecode::LdU16),
        1 => any::<u32>().prop_map(Bytecode::LdU32),
        1 => any::<U256>().prop_map(Bytecode::LdU256),
        ]
    }

    fn just_bytecode_strategy() -> impl Strategy<Value = Bytecode> {
        use Bytecode::*;

        static JUST_BYTECODES: &[Bytecode] = &[
            FreezeRef, Pop, Ret, LdTrue, LdFalse, ReadRef, WriteRef, Add, Sub, Mul, Mod, Div,
            BitOr, BitAnd, Xor, Or, And, Eq, Neq, Lt, Gt, Le, Ge, Abort, CastU8, CastU64, CastU128,
            CastU16, CastU32, CastU256, Not, Nop, Shl, Shr,
        ];
        select(JUST_BYTECODES)
    }
}
