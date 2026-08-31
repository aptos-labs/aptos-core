// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Analysis which computes information needed in backends for monomorphization. This
//! computes the distinct type instantiations in the model for structs and inlined functions.

use itertools::Itertools;
use move_core_types::{account_address::AccountAddress, function::ClosureMask};
use move_model::{
    ast,
    ast::{Address, Condition, ConditionKind, ExpData},
    model::{
        FieldEnv, FunId, GlobalEnv, ModuleId, Parameter, QualifiedId, QualifiedInstId, SpecFunId,
        SpecVarId, StructEnv, StructId,
    },
    pragmas::{
        INTRINSIC_FUN_MAP_BACK_KEY, INTRINSIC_FUN_MAP_BORROW_BACK, INTRINSIC_FUN_MAP_BORROW_FRONT,
        INTRINSIC_FUN_MAP_FRONT_KEY, INTRINSIC_FUN_MAP_GET, INTRINSIC_FUN_MAP_ITER_BORROW_MUT,
        INTRINSIC_FUN_MAP_KEYS, INTRINSIC_FUN_MAP_NEW_FROM, INTRINSIC_FUN_MAP_NEXT_KEY,
        INTRINSIC_FUN_MAP_POP_BACK, INTRINSIC_FUN_MAP_POP_FRONT, INTRINSIC_FUN_MAP_PREV_KEY,
        INTRINSIC_FUN_MAP_REMOVE_OR_NONE, INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD_ALL, INTRINSIC_FUN_MAP_SPEC_ABORTS_APPEND_DISJOINT,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW, INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY, INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_MUT, INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_FROM,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_WITH_CONFIG,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_REPLACE_KEY_INPLACE, INTRINSIC_FUN_MAP_SPEC_ABORTS_TRIM,
        INTRINSIC_FUN_MAP_SPEC_ABORTS_UPSERT_ALL, INTRINSIC_FUN_MAP_SPEC_DEL,
        INTRINSIC_FUN_MAP_SPEC_GET, INTRINSIC_FUN_MAP_SPEC_HAS_KEY,
        INTRINSIC_FUN_MAP_SPEC_IS_EMPTY, INTRINSIC_FUN_MAP_SPEC_ITER_PRESERVED,
        INTRINSIC_FUN_MAP_SPEC_ITER_VALID, INTRINSIC_FUN_MAP_SPEC_KEY_AT,
        INTRINSIC_FUN_MAP_SPEC_LEAF_ITER_VALID, INTRINSIC_FUN_MAP_SPEC_LEAF_OFFSET,
        INTRINSIC_FUN_MAP_SPEC_LEN, INTRINSIC_FUN_MAP_SPEC_NEW, INTRINSIC_FUN_MAP_SPEC_RANK,
        INTRINSIC_FUN_MAP_SPEC_SET, INTRINSIC_FUN_MAP_TO_ORDERED_MAP,
        INTRINSIC_FUN_MAP_TO_VEC_PAIR, INTRINSIC_FUN_MAP_UPSERT, INTRINSIC_TYPE_MAP,
        INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS,
    },
    symbol::Symbol,
    ty::{NoUnificationContext, PrimitiveType, ReferenceKind, Type, TypeDisplayContext, Variance},
    ty_invariant_analysis::{TypeInstantiationDerivation, TypeUnificationAdapter},
    well_known::{
        TYPE_INFO_MOVE, TYPE_INFO_SPEC, TYPE_NAME_GET_MOVE, TYPE_NAME_GET_SPEC, TYPE_NAME_MOVE,
        TYPE_NAME_SPEC, TYPE_SPEC_IS_STRUCT,
    },
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{BorrowEdge, Bytecode, Operation},
    usage_analysis::UsageProcessor,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    rc::Rc,
};

/// The environment extension computed by this analysis.
#[derive(Clone, Default, Debug)]
pub struct MonoInfo {
    pub structs: BTreeMap<QualifiedId<StructId>, BTreeSet<Vec<Type>>>,
    pub funs: BTreeMap<(QualifiedId<FunId>, FunctionVariant), BTreeSet<Vec<Type>>>,
    pub spec_funs: BTreeMap<QualifiedId<SpecFunId>, BTreeSet<Vec<Type>>>,
    pub move_equality_congruence_spec_funs: BTreeSet<QualifiedInstId<SpecFunId>>,
    pub spec_vars: BTreeMap<QualifiedId<SpecVarId>, BTreeSet<Vec<Type>>>,
    pub type_params: BTreeSet<u16>,
    pub vec_inst: BTreeSet<Type>,
    pub tuple_inst: BTreeSet<Vec<Type>>,
    pub table_inst: BTreeMap<QualifiedId<StructId>, BTreeSet<(Type, Type)>>,
    pub native_inst: BTreeMap<ModuleId, BTreeSet<Vec<Type>>>,
    /// Type instantiations observed at calls to intrinsic/native functions, keyed by
    /// callee. `info.funs` isn't populated for these callees, so post-passes use this
    /// map to ask which intrinsic roles were called and with which type args.
    pub intrinsic_calls: BTreeMap<QualifiedId<FunId>, BTreeSet<Vec<Type>>>,
    pub all_types: BTreeSet<Type>,
    pub axioms: Vec<(Condition, Vec<Vec<Type>>)>,
    /// A map from function types used in the program to the closures appearing in
    /// code constructing values of this function type.
    pub fun_infos: BTreeMap<Type, BTreeSet<ClosureInfo>>,
    /// Function types used by a behavioral predicate in the analyzed program.
    pub behavioral_fun_types: BTreeSet<Type>,
    /// Function types invoked dynamically in the analyzed program.
    pub applied_fun_types: BTreeSet<Type>,
    /// A map from function types to function-typed parameters of verification target functions.
    /// This enables the Boogie backend to generate parameter variants in the function type datatype.
    pub fun_param_infos: BTreeMap<Type, BTreeSet<FunParamInfo>>,
    /// A map from function types to struct fields containing storable function values.
    /// This enables the Boogie backend to generate struct field variants in the function type
    /// datatype with uninterpreted behavioral predicates.
    pub fun_struct_field_infos: BTreeMap<Type, BTreeSet<StructFieldInfo>>,
    /// Semantic slice for the selected verification root. Concrete struct
    /// instances localize validity and equality definitions in the Boogie backend.
    pub root_slices: BTreeMap<VerificationRoot, MonoSlice>,
}

/// A concrete verification entry point emitted by the Boogie backend.
#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct VerificationRoot {
    pub fun: QualifiedId<FunId>,
    pub variant: FunctionVariant,
    pub inst: Vec<Type>,
}

/// Semantic instances reachable from one verification root.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct MonoSlice {
    pub structs: BTreeMap<QualifiedId<StructId>, BTreeSet<Vec<Type>>>,
}

impl MonoInfo {
    /// Returns the set of all resource (key-ability) struct instantiations.
    /// This is the set of memory types that the Boogie backend will translate,
    /// and the correct scope for wildcard `reads_of<f> *` / `modifies_of<f> *`.
    pub fn all_memory_qids(&self, env: &GlobalEnv) -> BTreeSet<QualifiedInstId<StructId>> {
        let mut result = BTreeSet::new();
        for (sid, insts) in &self.structs {
            let struct_env = env.get_struct(*sid);
            if struct_env.has_memory() {
                for inst in insts {
                    result.insert(sid.instantiate(inst.clone()));
                }
            }
        }
        result
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct ClosureInfo {
    /// The function used to construct the closure.
    pub fun: QualifiedInstId<FunId>,
    /// Closure mask used by the function.
    pub mask: ClosureMask,
}

/// Information about a function parameter that has function type.
/// This is used to track function-typed parameters in verification targets
/// so that the Boogie backend can generate appropriate variants.
#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct FunParamInfo {
    /// The function containing this parameter.
    pub fun: QualifiedInstId<FunId>,
    /// The parameter name/symbol.
    pub param_sym: Symbol,
}

/// Information about a struct field that has a storable function type.
/// This is used to track function-valued fields in structs so the Boogie backend
/// can generate appropriate datatype variants with uninterpreted behavioral predicates.
#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
pub struct StructFieldInfo {
    /// The instantiated struct containing this function-typed field.
    pub struct_id: QualifiedInstId<StructId>,
    /// The field name.
    pub field_sym: Symbol,
}

/// Get the information computed by this analysis.
pub fn get_info(env: &GlobalEnv) -> Rc<MonoInfo> {
    env.get_extension::<MonoInfo>()
        .unwrap_or_else(|| Rc::new(MonoInfo::default()))
}

pub struct MonoAnalysisProcessor();

impl MonoAnalysisProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self())
    }

    /// Compute monomorphization information for one already-transformed
    /// verification root without replacing the package-wide environment
    /// extension.
    pub fn analyze_for_root(
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
        root: VerificationRoot,
    ) -> MonoInfo {
        Self::compute(env, targets, Some(root))
    }

    fn compute(
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
        root: Option<VerificationRoot>,
    ) -> MonoInfo {
        let mut analyzer = Analyzer {
            env,
            targets,
            info: MonoInfo::default(),
            todo_funs: vec![],
            done_funs: BTreeSet::new(),
            todo_spec_funs: vec![],
            done_spec_funs: BTreeSet::new(),
            done_function_specs: BTreeSet::new(),
            done_types: BTreeSet::new(),
            inst_opt: None,
            current_node: None,
            node_deps: BTreeMap::new(),
            node_types: BTreeMap::new(),
            selected_root: root,
        };
        // Analyze axioms found in modules.
        for module_env in env.get_modules() {
            for axiom in module_env.get_spec().filter_kind_axiom() {
                analyzer.analyze_exp(&axiom.exp)
            }
        }
        analyzer.analyze_funs();
        analyzer.register_intrinsic_associated_types();
        if analyzer.selected_root.is_some() {
            analyzer.info.root_slices = analyzer.compute_root_slices();
        }
        let Analyzer {
            mut info,
            done_types,
            ..
        } = analyzer;
        info.all_types = done_types;
        info
    }
}

/// This processor computes monomorphization information for backends.
impl FunctionTargetProcessor for MonoAnalysisProcessor {
    fn name(&self) -> String {
        "mono_analysis".to_owned()
    }

    fn is_single_run(&self) -> bool {
        true
    }

    fn run(&self, env: &GlobalEnv, targets: &mut FunctionTargetsHolder) {
        self.analyze(env, targets);
    }

    fn dump_result(
        &self,
        f: &mut fmt::Formatter,
        env: &GlobalEnv,
        _targets: &FunctionTargetsHolder,
    ) -> fmt::Result {
        writeln!(f, "\n\n==== mono-analysis result ====\n")?;
        let info = env
            .get_extension::<MonoInfo>()
            .expect("monomorphization analysis not run");
        let tctx = TypeDisplayContext::new(env);
        let display_inst = |tys: &[Type]| {
            tys.iter()
                .map(|ty| ty.display(&tctx).to_string())
                .join(", ")
        };
        for (sid, insts) in &info.structs {
            let sname = env.get_struct(*sid).get_full_name_str();
            writeln!(f, "struct {} = {{", sname)?;
            for inst in insts {
                writeln!(f, "  <{}>", display_inst(inst))?;
            }
            writeln!(f, "}}")?;
        }
        for ((fid, variant), insts) in &info.funs {
            let fname = env.get_function(*fid).get_full_name_str();
            writeln!(f, "fun {} [{}] = {{", fname, variant)?;
            for inst in insts {
                writeln!(f, "  <{}>", display_inst(inst))?;
            }
            writeln!(f, "}}")?;
        }
        for (fid, insts) in &info.spec_funs {
            let module_env = env.get_module(fid.module_id);
            let decl = module_env.get_spec_fun(fid.id);
            let mname = module_env.get_full_name_str();
            let fname = decl.name.display(env.symbol_pool());
            writeln!(f, "spec fun {}::{} = {{", mname, fname)?;
            for inst in insts {
                writeln!(f, "  <{}>", display_inst(inst))?;
            }
            writeln!(f, "}}")?;
        }
        for (module, insts) in &info.native_inst {
            writeln!(
                f,
                "module {} = {{",
                env.get_module(*module).get_full_name_str()
            )?;
            for inst in insts {
                writeln!(f, "  <{}>", display_inst(inst))?;
            }
            writeln!(f, "}}")?;
        }
        for (cond, insts) in &info.axioms {
            writeln!(f, "axiom {} = {{", cond.loc.display(env))?;
            for inst in insts {
                writeln!(f, "  <{}>", display_inst(inst))?;
            }
            writeln!(f, "}}")?;
        }
        for (root, slice) in &info.root_slices {
            let fname = env.get_function(root.fun).get_full_name_str();
            writeln!(
                f,
                "root {} [{}] <{}> = {{",
                fname,
                root.variant,
                display_inst(&root.inst)
            )?;
            for (sid, insts) in &slice.structs {
                let sname = env.get_struct(*sid).get_full_name_str();
                for inst in insts {
                    writeln!(f, "  {}<{}>", sname, display_inst(inst))?;
                }
            }
            writeln!(f, "}}")?;
        }

        Ok(())
    }
}

// Instantiation Analysis
// ======================

impl MonoAnalysisProcessor {
    fn analyze<'a>(&self, env: &'a GlobalEnv, targets: &'a FunctionTargetsHolder) {
        env.set_extension(Self::compute(env, targets, None));
    }
}

struct Analyzer<'a> {
    env: &'a GlobalEnv,
    targets: &'a FunctionTargetsHolder,
    info: MonoInfo,
    todo_funs: Vec<(QualifiedId<FunId>, FunctionVariant, Vec<Type>)>,
    done_funs: BTreeSet<(QualifiedId<FunId>, FunctionVariant, Vec<Type>)>,
    todo_spec_funs: Vec<(QualifiedId<SpecFunId>, Vec<Type>)>,
    done_spec_funs: BTreeSet<(QualifiedId<SpecFunId>, Vec<Type>)>,
    done_function_specs: BTreeSet<QualifiedInstId<FunId>>,
    done_types: BTreeSet<Type>,
    inst_opt: Option<Vec<Type>>,
    current_node: Option<MonoNode>,
    node_deps: BTreeMap<MonoNode, BTreeSet<MonoNode>>,
    node_types: BTreeMap<MonoNode, BTreeSet<Type>>,
    selected_root: Option<VerificationRoot>,
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq)]
enum MonoNode {
    Fun(QualifiedId<FunId>, FunctionVariant, Vec<Type>),
    SpecFun(QualifiedId<SpecFunId>, Vec<Type>),
}

/// Locate the `0x1::option::Option` struct. Pinned to `0x1` because the backend
/// hard-codes `$1_option_*` Boogie symbols.
fn find_option_struct(env: &GlobalEnv) -> Option<QualifiedId<StructId>> {
    let option_module_sym = env.symbol_pool().make("option");
    let option_struct_sym = env.symbol_pool().make("Option");
    let std_addr = Address::Numerical(AccountAddress::ONE);
    for module in env.get_modules() {
        let name = module.get_name();
        if name.addr() == &std_addr && name.name() == option_module_sym {
            if let Some(sid) = module.find_struct(option_struct_sym).map(|s| s.get_id()) {
                return Some(module.get_id().qualified(sid));
            }
        }
    }
    None
}

/// Locate the `0x1::cmp` module. Pinned to `0x1` because the backend hard-codes
/// `$1_cmp_*` Boogie symbols.
fn find_cmp_module(env: &GlobalEnv) -> Option<ModuleId> {
    let cmp_sym = env.symbol_pool().make("cmp");
    let std_addr = Address::Numerical(AccountAddress::ONE);
    for module in env.get_modules() {
        let name = module.get_name();
        if name.addr() == &std_addr && name.name() == cmp_sym {
            return Some(module.get_id());
        }
    }
    None
}

/// How a data invariant can observe hidden validity slots: by (transitively)
/// calling a bound validity predicate, or by lifting a function's spec
/// conditions through a behavioral predicate.
#[derive(Clone, Copy)]
enum DataInvOffense {
    Role(&'static str),
    Behavioral,
}

impl Analyzer<'_> {
    /// Register companion types needed by intrinsic role templates that no Move source
    /// references directly:
    ///  - `Option<V>` (eager) for roles building Option results (`map_upsert`,
    ///    `map_remove_or_none`, `map_get`).
    ///  - `vector<K>` (eager) for roles returning key vectors (`map_keys`,
    ///    `map_to_vec_pair`, `map_new_from`) so `$ContainsVec'K'` gets emitted.
    ///  - `cmp::compare<K>` and `Option<K>` (lazy) for ordering roles — registered only
    ///    when `intrinsic_calls` records an actual call with K.
    ///  - Abort-condition spec funs (`spec_aborts_empty`, `spec_aborts_add_all`, ...)
    ///    referenced from `abort_spec_fun` intrinsic pragmas — needed so
    ///    `SpecTranslator::translate_spec_funs` emits their declarations when a caller
    ///    uses `aborts_of<f>(...)`. Native spec funs (simple_map) are still safe to
    ///    include: `translate_spec_fun` skips them (their bodies come from the prelude).
    fn register_intrinsic_associated_types(&mut self) {
        let option_qid = find_option_struct(self.env);
        let cmp_mid = find_cmp_module(self.env);
        let intrinsics = self.env.get_intrinsics();
        let option_v_roles = [
            INTRINSIC_FUN_MAP_UPSERT,
            INTRINSIC_FUN_MAP_REMOVE_OR_NONE,
            INTRINSIC_FUN_MAP_GET,
        ];
        let cmp_k_roles = [
            INTRINSIC_FUN_MAP_BORROW_FRONT,
            INTRINSIC_FUN_MAP_BORROW_BACK,
            INTRINSIC_FUN_MAP_FRONT_KEY,
            INTRINSIC_FUN_MAP_BACK_KEY,
            INTRINSIC_FUN_MAP_POP_FRONT,
            INTRINSIC_FUN_MAP_POP_BACK,
            INTRINSIC_FUN_MAP_PREV_KEY,
            INTRINSIC_FUN_MAP_NEXT_KEY,
            // keys/to_vec_pair state key-vector sortedness, which needs
            // `cmp::compare<K>` (the clause is gated on `cmp_available`).
            INTRINSIC_FUN_MAP_KEYS,
            INTRINSIC_FUN_MAP_TO_VEC_PAIR,
        ];
        // Option<K> tracks all cmp roles (not just prev/next): prev_key/next_key
        // templates are emitted whenever `cmp_available` is set, and reference `Option<K>`.
        let option_k_roles_lazy = cmp_k_roles;
        let vec_k_roles_eager = [
            INTRINSIC_FUN_MAP_KEYS,
            INTRINSIC_FUN_MAP_TO_VEC_PAIR,
            INTRINSIC_FUN_MAP_NEW_FROM,
        ];
        let abort_spec_fun_roles = [
            INTRINSIC_FUN_MAP_SPEC_ABORTS_EMPTY,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD_ALL,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_FROM,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_NEW_WITH_CONFIG,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_APPEND_DISJOINT,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_TRIM,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_UPSERT_ALL,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_REPLACE_KEY_INPLACE,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW,
            INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_MUT,
        ];
        let mut option_v_to_register: Vec<Type> = vec![];
        let mut option_k_to_register: Vec<Type> = vec![];
        let mut cmp_k_to_register: Vec<Type> = vec![];
        let mut vec_k_to_register: Vec<Type> = vec![];
        let mut iter_ptr_to_register: Vec<Type> = vec![];
        let mut spec_fun_to_register: Vec<(QualifiedId<SpecFunId>, Vec<Type>)> = vec![];
        let mut validity_preds: BTreeMap<QualifiedId<SpecFunId>, &'static str> = BTreeMap::new();
        for (struct_qid, ty_args) in self.info.table_inst.iter() {
            let Some(decl) = intrinsics.get_decl_for_struct(struct_qid) else {
                continue;
            };
            let needs_option_v = option_v_roles
                .iter()
                .any(|name| decl.get_fun_triple(self.env, name).is_some());
            if needs_option_v {
                for (_k, v) in ty_args.iter() {
                    option_v_to_register.push(v.clone());
                }
            }
            let needs_vec_k = vec_k_roles_eager
                .iter()
                .any(|name| decl.get_fun_triple(self.env, name).is_some());
            if needs_vec_k {
                for (k, _v) in ty_args.iter() {
                    vec_k_to_register.push(k.clone());
                }
            }
            // to_vec_pair's template also references `$IsValid'vec<V>'` for the
            // values vector, so register vec<V> eagerly as well.
            if decl
                .lookup_move_fun(self.env, INTRINSIC_FUN_MAP_TO_VEC_PAIR)
                .is_some()
            {
                for (_k, v) in ty_args.iter() {
                    vec_k_to_register.push(v.clone());
                }
            }
            // iter_borrow_mut's template references the iterator enum `Iter<K>`
            // (its first parameter type), so register it eagerly per instance.
            if let Some(fun_qid) = decl.lookup_move_fun(self.env, INTRINSIC_FUN_MAP_ITER_BORROW_MUT)
            {
                let fun_env = self.env.get_function(fun_qid);
                let param_tys = fun_env.get_parameter_types();
                // The template has a fixed `(self: Iter<K>, m: $Mutation Map<K,V>)
                // returns ($Mutation V, $Mutation Map)` shape specialized only by
                // `K`/`V`: the binding must have exactly two parameters and two
                // type parameters, and take the iterator BY VALUE (the template
                // parameter is by value; a `&mut Iter<K>` binding would pass a
                // `$Mutation` into a value slot). Extra parameters or a wider
                // type-parameter list would make emitted calls arity-mismatched.
                if param_tys.len() != 2 || fun_env.get_type_parameters().len() != 2 {
                    self.env.error(
                        &fun_env.get_loc(),
                        "a `map_iter_borrow_mut` function must take exactly the \
                         iterator and the map as parameters and have exactly two \
                         type parameters (key and value)",
                    );
                } else if param_tys.first().is_some_and(Type::is_reference) {
                    self.env.error(
                        &fun_env.get_loc(),
                        "a `map_iter_borrow_mut` function must take its iterator \
                         enum by value as the first parameter",
                    );
                } else if let Some(Type::Struct(mid, sid, inst)) =
                    param_tys.first().map(|ty| ty.skip_reference())
                {
                    // The fabricated instantiation mirrors the enum's own arity:
                    // a keyed iterator is `Iter<K>`, a position-based one is
                    // unparameterized. Anything wider would index its argument
                    // list out of bounds, so reject it with a diagnostic rather
                    // than crashing.
                    let iter_ty_params = self
                        .env
                        .get_struct(mid.qualified(*sid))
                        .get_type_parameters()
                        .len();
                    if iter_ty_params > 1 {
                        self.env.error(
                            &fun_env.get_loc(),
                            "the iterator enum of a `map_iter_borrow_mut` function must \
                             have at most one type parameter (the key type)",
                        );
                    } else if iter_ty_params == 1 && inst.first() != Some(&Type::TypeParameter(0)) {
                        // The template hardcodes the map instance's key type
                        // for the iterator parameter; a binding instantiated
                        // with anything but the function's first (key) type
                        // parameter would make the emitted call ill-typed.
                        self.env.error(
                            &fun_env.get_loc(),
                            "the iterator parameter of a `map_iter_borrow_mut` function \
                             must be instantiated with the function's first (key) type \
                             parameter",
                        );
                    } else if !{
                        // The template's write-back updates the abstract table
                        // at the iterator key through the RETURNED reference:
                        // the binding must take `&mut` of the declaring map
                        // type (instantiated with the function's type
                        // parameters in order) and return `&mut` of the value
                        // type parameter — otherwise a verified caller could
                        // prove a table-value update the runtime applies to
                        // unrelated state.
                        let expected_map = Type::Struct(
                            decl.get_move_type().module_id,
                            decl.get_move_type().id,
                            vec![Type::TypeParameter(0), Type::TypeParameter(1)],
                        );
                        let map_ok = matches!(
                            param_tys.get(1),
                            Some(Type::Reference(ReferenceKind::Mutable, inner))
                                if **inner == expected_map
                        );
                        let ret_ok = matches!(
                            &fun_env.get_result_type(),
                            Type::Reference(ReferenceKind::Mutable, inner)
                                if **inner == Type::TypeParameter(1)
                        );
                        map_ok && ret_ok
                    } {
                        self.env.error(
                            &fun_env.get_loc(),
                            "a `map_iter_borrow_mut` function must take a mutable \
                             reference to the intrinsic map (instantiated with the \
                             function's type parameters) and return a mutable reference \
                             to the value type parameter",
                        );
                    } else if iter_ty_params == 1 {
                        for (k, _v) in ty_args.iter() {
                            iter_ptr_to_register.push(Type::Struct(*mid, *sid, vec![k.clone()]));
                        }
                    } else {
                        // Unparameterized iterator: one instance covers every
                        // map instance.
                        iter_ptr_to_register.push(Type::Struct(*mid, *sid, vec![]));
                    }
                }
            }
            // The abort predicate for `map_iter_borrow_mut` is called by the
            // behavioral `aborts_of` translation with exactly the Move
            // function's two parameters, whatever the binding's own signature
            // — so validate it for every binding kind, not only the natives
            // the template defines. The iterator type comes from the
            // `map_iter_borrow_mut` binding, which must be co-bound.
            if let Some(sf_qid) =
                decl.lookup_spec_fun(self.env, INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_MUT)
            {
                let module_env = self.env.get_module(sf_qid.module_id);
                let sf = module_env.get_spec_fun(sf_qid.id);
                match decl.lookup_move_fun(self.env, INTRINSIC_FUN_MAP_ITER_BORROW_MUT) {
                    None => {
                        self.env.error(
                            &sf.loc,
                            "a `map_spec_aborts_iter_borrow_mut` binding requires \
                             `map_iter_borrow_mut` bound on the same map",
                        );
                    },
                    Some(fun_qid) => {
                        let iter_ty = self
                            .env
                            .get_function(fun_qid)
                            .get_parameter_types()
                            .first()
                            .map(|ty| ty.skip_reference().clone());
                        if matches!(iter_ty, Some(Type::Struct(..))) {
                            let expected_map = Type::Struct(
                                decl.get_move_type().module_id,
                                decl.get_move_type().id,
                                vec![Type::TypeParameter(0), Type::TypeParameter(1)],
                            );
                            if sf.type_params.len() != 2
                                || sf.params.len() != 2
                                || Some(&sf.params[0].1) != iter_ty.as_ref()
                                || sf.params[1].1 != expected_map
                                || sf.result_type != Type::Primitive(PrimitiveType::Bool)
                            {
                                self.env.error(
                                    &sf.loc,
                                    "a `map_spec_aborts_iter_borrow_mut` function must \
                                     have two type parameters and the signature \
                                     (iterator_enum<K>, map<K, V>): bool, with the \
                                     iterator enum of the `map_iter_borrow_mut` binding",
                                );
                            }
                        }
                    },
                }
            }
            // Template-defined roles have fixed signatures: the template
            // emits the definition in that shape while user spec text calls
            // the binding as declared — a deviating declaration makes the
            // two disagree in emitted Boogie.
            {
                let map_ty = || {
                    Type::Struct(
                        decl.get_move_type().module_id,
                        decl.get_move_type().id,
                        vec![Type::TypeParameter(0), Type::TypeParameter(1)],
                    )
                };
                let k = || Type::TypeParameter(0);
                let v = || Type::TypeParameter(1);
                let num = || Type::Primitive(PrimitiveType::Num);
                let boolean = || Type::Primitive(PrimitiveType::Bool);
                let fixed_sigs: [(&str, Vec<Type>, Type, &str); 13] = [
                    (
                        INTRINSIC_FUN_MAP_SPEC_NEW,
                        vec![],
                        map_ty(),
                        "(): map<K, V>",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_LEN,
                        vec![map_ty()],
                        num(),
                        "(map<K, V>): num",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_KEY_AT,
                        vec![map_ty(), num()],
                        k(),
                        "(map<K, V>, num): K",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_RANK,
                        vec![map_ty(), k()],
                        num(),
                        "(map<K, V>, K): num",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_IS_EMPTY,
                        vec![map_ty()],
                        boolean(),
                        "(map<K, V>): bool",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_HAS_KEY,
                        vec![map_ty(), k()],
                        boolean(),
                        "(map<K, V>, K): bool",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_GET,
                        vec![map_ty(), k()],
                        v(),
                        "(map<K, V>, K): V",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_SET,
                        vec![map_ty(), k(), v()],
                        map_ty(),
                        "(map<K, V>, K, V): map<K, V>",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_DEL,
                        vec![map_ty(), k()],
                        map_ty(),
                        "(map<K, V>, K): map<K, V>",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY,
                        vec![map_ty()],
                        boolean(),
                        "(map<K, V>): bool",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD,
                        vec![map_ty(), k(), v()],
                        boolean(),
                        "(map<K, V>, K, V): bool",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL,
                        vec![map_ty(), k()],
                        boolean(),
                        "(map<K, V>, K): bool",
                    ),
                    (
                        INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW,
                        vec![map_ty(), k()],
                        boolean(),
                        "(map<K, V>, K): bool",
                    ),
                ];
                for (role, exp_params, exp_result, shape) in fixed_sigs {
                    let Some(sf_qid) = decl.lookup_spec_fun(self.env, role) else {
                        continue;
                    };
                    let module_env = self.env.get_module(sf_qid.module_id);
                    let sf = module_env.get_spec_fun(sf_qid.id);
                    if sf.type_params.len() != 2
                        || sf.params.len() != exp_params.len()
                        || sf
                            .params
                            .iter()
                            .zip(exp_params.iter())
                            .any(|(Parameter(_, ty, _), exp)| ty != exp)
                        || sf.result_type != exp_result
                    {
                        self.env.error(
                            &sf.loc,
                            &format!(
                                "a `{}` function must have two type parameters \
                                 and the signature {}",
                                role, shape
                            ),
                        );
                    }
                }
            }
            // Call-only abort predicates are defined by spec-function
            // translation (never by the template) and are invoked by the
            // behavioral `aborts_of` translation with exactly the paired
            // Move function's parameters — so the binding must not be a
            // `spec native fun`, and its signature must match the pairing.
            {
                let fixed_or_bespoke = [
                    INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY,
                    INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD,
                    INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL,
                    INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW,
                    INTRINSIC_FUN_MAP_SPEC_ABORTS_ITER_BORROW_MUT,
                ];
                let mut checked: BTreeSet<QualifiedId<SpecFunId>> = BTreeSet::new();
                for (role_name, fun_def) in INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS.iter() {
                    let Some(abort_name) = fun_def.abort_spec_fun else {
                        continue;
                    };
                    if fixed_or_bespoke.contains(&abort_name) {
                        continue;
                    }
                    let Some(sf_qid) = decl.lookup_spec_fun(self.env, abort_name) else {
                        continue;
                    };
                    let Some(fun_qid) = decl.lookup_move_fun(self.env, role_name) else {
                        continue;
                    };
                    if !checked.insert(sf_qid) {
                        continue;
                    }
                    let module_env = self.env.get_module(sf_qid.module_id);
                    let sf = module_env.get_spec_fun(sf_qid.id);
                    if sf.body.is_none() && !sf.uninterpreted {
                        self.env.error(
                            &sf.loc,
                            &format!(
                                "a `{}` binding must have a body or be declared \
                                 uninterpreted; the prover's model does not define it",
                                abort_name
                            ),
                        );
                        continue;
                    }
                    let fun_env = self.env.get_function(fun_qid);
                    let exp_params: Vec<Type> = fun_env
                        .get_parameter_types()
                        .iter()
                        .map(|ty| ty.skip_reference().clone())
                        .collect();
                    if sf.type_params.len() != fun_env.get_type_parameter_count()
                        || sf.params.len() != exp_params.len()
                        || sf
                            .params
                            .iter()
                            .zip(exp_params.iter())
                            .any(|(Parameter(_, ty, _), exp)| ty != exp)
                        || sf.result_type != Type::Primitive(PrimitiveType::Bool)
                    {
                        self.env.error(
                            &sf.loc,
                            &format!(
                                "a `{}` function must take the parameters of its \
                                 `{}` binding (references by value) and return bool",
                                abort_name, role_name
                            ),
                        );
                    }
                }
            }
            // Data invariants on the map itself are never applied: its
            // validity predicate is template-defined and its pack/unpack
            // sites are erased, so an `invariant` would be silently inert.
            {
                let struct_env = self.env.get_struct(*struct_qid);
                for cond in struct_env
                    .get_spec()
                    .filter_kind(ConditionKind::StructInvariant)
                {
                    self.env.error(
                        &cond.loc,
                        "data invariants are not supported on intrinsic map types; \
                         the map's validity is defined by its model",
                    );
                }
            }
            // Validity predicates have the fixed template shape
            // `(iterator_enum, map<K, V>): bool` with the iterator enum either
            // unparameterized (a key-agnostic walker) or instantiated with the
            // key type parameter; anything else would emit ill-typed Boogie.
            // Register the iterator enum's instances eagerly: the template
            // references them per map instance.
            // The validity predicates answer a yes/no question about a walker;
            // the leaf offset answers where it sits. Same parameter shape, so
            // the expected result type travels with the role.
            for (role, expected_result, result_name) in [
                (
                    INTRINSIC_FUN_MAP_SPEC_ITER_VALID,
                    Type::Primitive(PrimitiveType::Bool),
                    "bool",
                ),
                (
                    INTRINSIC_FUN_MAP_SPEC_LEAF_ITER_VALID,
                    Type::Primitive(PrimitiveType::Bool),
                    "bool",
                ),
                (
                    INTRINSIC_FUN_MAP_SPEC_LEAF_OFFSET,
                    Type::Primitive(PrimitiveType::Num),
                    "num",
                ),
            ] {
                let Some(sf_qid) = decl.lookup_spec_fun(self.env, role) else {
                    continue;
                };
                let module_env = self.env.get_module(sf_qid.module_id);
                let sf = module_env.get_spec_fun(sf_qid.id);
                let expected_map = Type::Struct(
                    decl.get_move_type().module_id,
                    decl.get_move_type().id,
                    vec![Type::TypeParameter(0), Type::TypeParameter(1)],
                );
                let iter_ok = matches!(
                    sf.params.first().map(|Parameter(_, ty, _)| ty),
                    Some(Type::Struct(_, _, inst))
                        if inst.is_empty() || inst[..] == [Type::TypeParameter(0)]
                );
                if sf.type_params.len() != 2
                    || sf.params.len() != 2
                    || !iter_ok
                    || sf.params[1].1 != expected_map
                    || sf.result_type != expected_result
                {
                    self.env.error(
                        &sf.loc,
                        &format!(
                            "a `{}` function must have two type parameters and the \
                             signature (iterator_enum, map<K, V>): {}, with the \
                             iterator enum either unparameterized or instantiated \
                             with the key type parameter",
                            role, result_name
                        ),
                    );
                    continue;
                }
                if let Some(Parameter(_, Type::Struct(imid, isid, iinst), _)) = sf.params.first() {
                    let generic = !iinst.is_empty();
                    for (k, _v) in ty_args.iter() {
                        iter_ptr_to_register.push(Type::Struct(
                            *imid,
                            *isid,
                            if generic { vec![k.clone()] } else { vec![] },
                        ));
                    }
                }
            }
            if let Some(sf_qid) =
                decl.lookup_spec_fun(self.env, INTRINSIC_FUN_MAP_SPEC_ITER_PRESERVED)
            {
                let module_env = self.env.get_module(sf_qid.module_id);
                let sf = module_env.get_spec_fun(sf_qid.id);
                let expected_map = Type::Struct(
                    decl.get_move_type().module_id,
                    decl.get_move_type().id,
                    vec![Type::TypeParameter(0), Type::TypeParameter(1)],
                );
                if sf.type_params.len() != 2
                    || sf.params.len() != 2
                    || sf.params[0].1 != expected_map
                    || sf.params[1].1 != expected_map
                    || sf.result_type != Type::Primitive(PrimitiveType::Bool)
                {
                    self.env.error(
                        &sf.loc,
                        "a `map_spec_iter_preserved` function must have two type \
                         parameters and the signature (map<K, V>, map<K, V>): bool",
                    );
                }
            }
            // Roles whose definitions the prelude template emits must bind
            // `spec native fun`s: they are the map model's own observers, and
            // for a bodied (or uninterpreted) spec fun the spec translator
            // emits a second Boogie function under the same name once any
            // spec uses it. (`map_spec_aborts_iter_borrow_mut` is exempt: its
            // template definition is gated on the binding being native, so a
            // bodied predicate supplies the abort semantics itself.)
            for role in [
                INTRINSIC_FUN_MAP_SPEC_NEW,
                INTRINSIC_FUN_MAP_SPEC_LEN,
                INTRINSIC_FUN_MAP_SPEC_IS_EMPTY,
                INTRINSIC_FUN_MAP_SPEC_HAS_KEY,
                INTRINSIC_FUN_MAP_SPEC_GET,
                INTRINSIC_FUN_MAP_SPEC_SET,
                INTRINSIC_FUN_MAP_SPEC_DEL,
                INTRINSIC_FUN_MAP_SPEC_KEY_AT,
                INTRINSIC_FUN_MAP_SPEC_RANK,
                INTRINSIC_FUN_MAP_SPEC_ABORTS_DESTROY_EMPTY,
                INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD,
                INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL,
                INTRINSIC_FUN_MAP_SPEC_ABORTS_BORROW,
                INTRINSIC_FUN_MAP_SPEC_ITER_VALID,
                INTRINSIC_FUN_MAP_SPEC_LEAF_ITER_VALID,
                INTRINSIC_FUN_MAP_SPEC_LEAF_OFFSET,
                INTRINSIC_FUN_MAP_SPEC_ITER_PRESERVED,
            ] {
                let Some(sf_qid) = decl.lookup_spec_fun(self.env, role) else {
                    continue;
                };
                let module_env = self.env.get_module(sf_qid.module_id);
                let sf = module_env.get_spec_fun(sf_qid.id);
                if sf.body.is_some() || sf.uninterpreted {
                    self.env.error(
                        &sf.loc,
                        &format!(
                            "a `{}` binding must be a `spec native fun`; \
                             its definition is provided by the intrinsic model",
                            role
                        ),
                    );
                }
            }
            // The enumeration roles define each other: the template emits both
            // declarations or neither, since `key_at`'s axioms are stated
            // through `rank` and vice versa. Binding only one would pass the
            // per-role checks above and then reach Boogie as a call to a
            // function that was never declared, so require the pair.
            {
                let key_at = decl.lookup_spec_fun(self.env, INTRINSIC_FUN_MAP_SPEC_KEY_AT);
                let rank = decl.lookup_spec_fun(self.env, INTRINSIC_FUN_MAP_SPEC_RANK);
                let missing = match (key_at, rank) {
                    (Some(qid), None) => Some((
                        qid,
                        INTRINSIC_FUN_MAP_SPEC_KEY_AT,
                        INTRINSIC_FUN_MAP_SPEC_RANK,
                    )),
                    (None, Some(qid)) => Some((
                        qid,
                        INTRINSIC_FUN_MAP_SPEC_RANK,
                        INTRINSIC_FUN_MAP_SPEC_KEY_AT,
                    )),
                    _ => None,
                };
                if let Some((sf_qid, bound_role, absent_role)) = missing {
                    let module_env = self.env.get_module(sf_qid.module_id);
                    let sf = module_env.get_spec_fun(sf_qid.id);
                    self.env.error(
                        &sf.loc,
                        &format!(
                            "`{}` is bound but `{}` is not; the two must be bound \
                             together, since the intrinsic model defines each \
                             through the other",
                            bound_role, absent_role
                        ),
                    );
                }
            }
            // `to_ordered_map`'s template returns the destination map as a raw
            // table. A destination type that declares ghost fields is
            // represented by a carrier the template does not construct, so
            // calls would be ill-typed; reject the binding.
            if let Some(fun_qid) = decl.lookup_move_fun(self.env, INTRINSIC_FUN_MAP_TO_ORDERED_MAP)
            {
                let fun_env = self.env.get_function(fun_qid);
                if let Type::Struct(dmid, dsid, _) = fun_env.get_result_type().skip_reference() {
                    if self
                        .env
                        .get_struct(dmid.qualified(*dsid))
                        .get_ghost_fields()
                        .next()
                        .is_some()
                    {
                        self.env.error(
                            &fun_env.get_loc(),
                            "a `map_to_ordered_map` function whose result map type \
                             carries iterator-validity state is not supported",
                        );
                    }
                }
            }
            for role in &cmp_k_roles {
                let Some(role_qid) = decl.lookup_move_fun(self.env, role) else {
                    continue;
                };
                let Some(call_actuals) = self.info.intrinsic_calls.get(&role_qid) else {
                    continue;
                };
                for actuals in call_actuals {
                    if let Some(k) = actuals.first() {
                        cmp_k_to_register.push(k.clone());
                    }
                }
            }
            for role in &option_k_roles_lazy {
                let Some(role_qid) = decl.lookup_move_fun(self.env, role) else {
                    continue;
                };
                let Some(call_actuals) = self.info.intrinsic_calls.get(&role_qid) else {
                    continue;
                };
                for actuals in call_actuals {
                    if let Some(k) = actuals.first() {
                        option_k_to_register.push(k.clone());
                    }
                }
            }
            for role in &abort_spec_fun_roles {
                let Some(spec_fun_qid) = decl.lookup_spec_fun(self.env, role) else {
                    continue;
                };
                for (k, v) in ty_args.iter() {
                    spec_fun_to_register.push((spec_fun_qid, vec![k.clone(), v.clone()]));
                }
            }
            for role in [
                INTRINSIC_FUN_MAP_SPEC_ITER_VALID,
                INTRINSIC_FUN_MAP_SPEC_LEAF_ITER_VALID,
                INTRINSIC_FUN_MAP_SPEC_ITER_PRESERVED,
            ] {
                if let Some(sf_qid) = decl.lookup_spec_fun(self.env, role) {
                    validity_preds.insert(sf_qid, role);
                }
            }
        }
        self.check_no_validity_preds_in_data_invariants(&validity_preds);
        // The backend marks a map's K as `cmp_available` when K appears in the
        // *contained-type closure* of any `cmp::compare` instance (the prelude
        // expands `native_inst[cmp]` with `get_all_contained_types_with_skip_reference`
        // because `compare` recurses into fields) — including instances this pass
        // itself adds for ordering roles, and direct user calls that never went
        // through an ordering role. The prev_key/next_key templates emit for every
        // cmp-available K and reference `Option<K>`, so mirror that closure here.
        if let Some(cmp_mid) = cmp_mid {
            let mut cmp_closure: BTreeSet<Type> = BTreeSet::new();
            let existing_cmp_ks = self
                .info
                .native_inst
                .get(&cmp_mid)
                .into_iter()
                .flatten()
                .filter_map(|inst| inst.first().cloned())
                .collect::<Vec<_>>();
            for ty in existing_cmp_ks.iter().chain(cmp_k_to_register.iter()) {
                cmp_closure.extend(ty.get_all_contained_types_with_skip_reference(self.env));
            }
            for (struct_qid, ty_args) in self.info.table_inst.iter() {
                let Some(decl) = intrinsics.get_decl_for_struct(struct_qid) else {
                    continue;
                };
                let prev_next_bound = [INTRINSIC_FUN_MAP_PREV_KEY, INTRINSIC_FUN_MAP_NEXT_KEY]
                    .iter()
                    .any(|name| decl.get_fun_triple(self.env, name).is_some());
                if prev_next_bound {
                    for (k, _v) in ty_args.iter() {
                        if cmp_closure.contains(k) {
                            option_k_to_register.push(k.clone());
                        }
                    }
                }
            }
        }
        if let Some(option_qid) = option_qid {
            for ty in option_v_to_register.into_iter().chain(option_k_to_register) {
                self.add_type(&Type::Struct(option_qid.module_id, option_qid.id, vec![ty]));
            }
        }
        for ty in vec_k_to_register {
            self.info.vec_inst.insert(ty);
        }
        for ty in iter_ptr_to_register {
            self.add_type(&ty);
        }
        if let Some(cmp_mid) = cmp_mid {
            for ty in cmp_k_to_register {
                self.info
                    .native_inst
                    .entry(cmp_mid)
                    .or_default()
                    .insert(vec![ty]);
            }
        }
        for (spec_fun_qid, ty_args) in spec_fun_to_register {
            self.info
                .spec_funs
                .entry(spec_fun_qid)
                .or_default()
                .insert(ty_args);
        }
    }

    /// Iterator-validity predicates must not appear in data invariants, on any
    /// type. A data invariant is assumed wherever a value of the type is
    /// assumed well-formed — in particular for opaque and native call results,
    /// where nothing ever proves it (intrinsic producers have no pack site).
    /// Since `map_spec_new`'s hidden slot is a fixed value, an invariant like
    /// `spec_iter_valid(self.it, spec_new())` would pin slots to a known
    /// constant and revalidate stale iterators after a structural mutation.
    /// Behavioral predicates are rejected wholesale in this context: a
    /// `requires_of<f>(..)` lifts `f`'s spec conditions — which may state
    /// validity, transitively and through function-typed targets this rung
    /// cannot resolve — into the same assumed position. The intrinsic map
    /// type itself is excluded: its data invariants are rejected wholesale by
    /// the per-declaration rung above.
    fn check_no_validity_preds_in_data_invariants(
        &self,
        validity_preds: &BTreeMap<QualifiedId<SpecFunId>, &'static str>,
    ) {
        if validity_preds.is_empty() {
            return;
        }
        let mut cache: BTreeMap<QualifiedId<SpecFunId>, Option<DataInvOffense>> = BTreeMap::new();
        for module in self.env.get_modules() {
            for struct_env in module.get_structs() {
                if struct_env.is_intrinsic_of(INTRINSIC_TYPE_MAP) {
                    continue;
                }
                for cond in struct_env
                    .get_spec()
                    .filter_kind(ConditionKind::StructInvariant)
                {
                    let offense =
                        self.data_inv_exp_offense(cond.exp.as_ref(), validity_preds, &mut cache);
                    if let Some(offense) = offense {
                        let msg = match offense {
                            DataInvOffense::Role(role) => format!(
                                "a `{}` binding cannot be used in a data invariant \
                                 (directly or through called spec functions): data \
                                 invariants are assumed for opaque results, which \
                                 would revalidate stale iterators",
                                role
                            ),
                            DataInvOffense::Behavioral => "a behavioral predicate \
                                 (`requires_of`, `aborts_of`, `ensures_of`, or `result_of`) \
                                 cannot be used in a data invariant when iterator-validity \
                                 bindings are present: it lifts a function's spec conditions \
                                 — possibly stating validity — into an assumed context"
                                .to_string(),
                        };
                        self.env.error(&cond.loc, &msg);
                    }
                }
            }
        }
    }

    /// Whether the expression can observe hidden validity slots: a call to a
    /// bound validity predicate (directly or through called spec functions'
    /// bodies), or any behavioral predicate. Memoized per spec function; the
    /// `None` pre-insertion doubles as a cycle guard for recursive spec funs.
    fn data_inv_exp_offense(
        &self,
        exp: &ExpData,
        validity_preds: &BTreeMap<QualifiedId<SpecFunId>, &'static str>,
        cache: &mut BTreeMap<QualifiedId<SpecFunId>, Option<DataInvOffense>>,
    ) -> Option<DataInvOffense> {
        let mut found = None;
        exp.visit_pre_order(&mut |e| {
            match e {
                ExpData::Call(_, ast::Operation::Behavior(..), _) => {
                    found = Some(DataInvOffense::Behavioral);
                },
                ExpData::Call(_, ast::Operation::SpecFunction(mid, fid, _), _) => {
                    found = self.spec_fun_offense(mid.qualified(*fid), validity_preds, cache);
                },
                _ => {},
            }
            found.is_none()
        });
        found
    }

    fn spec_fun_offense(
        &self,
        qid: QualifiedId<SpecFunId>,
        validity_preds: &BTreeMap<QualifiedId<SpecFunId>, &'static str>,
        cache: &mut BTreeMap<QualifiedId<SpecFunId>, Option<DataInvOffense>>,
    ) -> Option<DataInvOffense> {
        if let Some(role) = validity_preds.get(&qid) {
            return Some(DataInvOffense::Role(role));
        }
        if let Some(cached) = cache.get(&qid) {
            return *cached;
        }
        cache.insert(qid, None);
        let module_env = self.env.get_module(qid.module_id);
        let found = match &module_env.get_spec_fun(qid.id).body {
            Some(body) => self.data_inv_exp_offense(body.as_ref(), validity_preds, cache),
            None => None,
        };
        cache.insert(qid, found);
        found
    }

    fn analyze_funs(&mut self) {
        // Analyze top-level, verified functions. Any functions they call will be queued
        // in self.todo_targets for later analysis. During this phase, self.inst_opt is None.
        if let Some(root) = self.selected_root.clone() {
            let fun = self.env.get_function(root.fun);
            let target = self.targets.get_target(&fun, &root.variant);
            self.analyze_verification_root(root, target);
        } else {
            for module in self.env.get_modules() {
                for fun in module.get_functions() {
                    if fun.is_not_prover_target() {
                        continue;
                    }
                    for (variant, target) in self.targets.get_targets(&fun) {
                        if variant.is_verified() {
                            self.analyze_verification_root(
                                VerificationRoot {
                                    fun: fun.get_qualified_id(),
                                    variant,
                                    inst: vec![],
                                },
                                target,
                            );
                        }
                    }
                }
            }
        }

        // Next do todo-list for regular functions, while self.inst_opt contains the
        // specific instantiation.
        while let Some((fun, variant, inst)) = self.todo_funs.pop() {
            self.current_node = Some(MonoNode::Fun(fun, variant.clone(), inst.clone()));
            self.inst_opt = Some(inst);
            self.analyze_fun(
                self.targets
                    .get_target(&self.env.get_function(fun), &variant),
            );
            let inst = std::mem::take(&mut self.inst_opt).unwrap();
            // Insert it into final analysis result.
            self.info
                .funs
                .entry((fun, variant.clone()))
                .or_default()
                .insert(inst.clone());
            self.done_funs.insert((fun, variant, inst));
            self.current_node = None;
        }

        // Next do axioms, based on the types discovered for regular functions.
        let axioms = self.compute_axiom_instances();
        for (cond, insts) in axioms {
            for inst in &insts {
                self.inst_opt = Some(inst.clone());
                self.analyze_exp(&cond.exp);
            }
            self.info.axioms.push((cond, insts))
        }

        // Finally do spec functions, after all regular functions and axioms are done.
        while let Some((fun, inst)) = self.todo_spec_funs.pop() {
            self.current_node = Some(MonoNode::SpecFun(fun, inst.clone()));
            self.inst_opt = Some(inst);
            self.analyze_spec_fun(fun);
            let inst = std::mem::take(&mut self.inst_opt).unwrap();
            // Insert it into final analysis result.
            self.info
                .spec_funs
                .entry(fun)
                .or_default()
                .insert(inst.clone());
            self.done_spec_funs.insert((fun, inst));
            self.current_node = None;
        }
    }

    fn analyze_verification_root(&mut self, root: VerificationRoot, target: FunctionTarget<'_>) {
        if !root.inst.is_empty() {
            self.info
                .funs
                .entry((root.fun, root.variant.clone()))
                .or_default()
                .insert(root.inst.clone());
        }
        self.current_node = Some(MonoNode::Fun(
            root.fun,
            root.variant.clone(),
            root.inst.clone(),
        ));
        self.inst_opt = (!root.inst.is_empty()).then(|| root.inst.clone());
        self.analyze_fun(target.clone());

        // Modify targets are not represented in the bytecode.
        for (memory, exps) in target.get_modify_ids_and_exps() {
            self.add_type_root(&memory.to_type());
            for exp in exps {
                self.analyze_exp(&exp);
            }
        }
        self.inst_opt = None;
        self.current_node = None;
    }

    /// Analyze axioms, computing all the instantiations needed. We over-approximate the
    /// instantiations by using the cartesian product of all known types. As the number of
    /// type parameters for axioms is restricted to 2, the number of instantiations
    /// should stay in range. Since each axiom instance is eventually instantiated for
    /// distinct types, unnecessary axioms should be ignorable by the SMT solver, avoiding
    /// over-triggering.
    fn compute_axiom_instances(&self) -> Vec<(Condition, Vec<Vec<Type>>)> {
        let mut axioms = vec![];
        let all_types = self
            .done_types
            .iter()
            .filter(|t| t.can_be_type_argument())
            .cloned()
            .collect::<Vec<_>>();
        for module_env in self.env.get_modules() {
            for cond in &module_env.get_spec().conditions {
                if let ConditionKind::Axiom(params) = &cond.kind {
                    let type_insts = match params.len() {
                        0 => vec![vec![]],
                        1 => all_types.iter().cloned().map(|t| vec![t]).collect(),
                        2 => itertools::iproduct!(
                            all_types.iter().cloned(),
                            all_types.iter().cloned()
                        )
                        .map(|(x, y)| vec![x, y])
                        .collect(),
                        _ => {
                            self.env.error(
                                &cond.loc,
                                "axioms cannot have more than two type parameters",
                            );
                            vec![]
                        },
                    };
                    axioms.push((cond.clone(), type_insts));
                }
            }
        }
        axioms
    }

    fn analyze_fun(&mut self, target: FunctionTarget<'_>) {
        // Analyze function locals and return value types.
        for idx in 0..target.get_local_count() {
            self.add_type_root(target.get_local_type(idx));
        }
        for ty in target.get_return_types().iter() {
            self.add_type_root(ty);
        }
        // Analyze code.
        if !target.func_env.is_native_or_intrinsic() {
            for bc in target.get_bytecode() {
                self.analyze_bytecode(&target, bc);
            }
        }
        // Analyze spec conditions and proof hints for closures and types.
        {
            let spec = target.get_spec();
            for cond in spec.conditions.iter() {
                for exp in cond.all_exps() {
                    self.analyze_exp(exp);
                }
            }
            for exp in spec.proof_exps() {
                self.analyze_exp(exp);
            }
        }
        // Analyze behavioral parameters when this function is a verification root.
        // A selected concrete root has an instantiation, while package-wide analysis
        // reaches this block with no instantiation.
        let analyzing_selected_root = self.selected_root.as_ref().is_some_and(|root| {
            self.current_node
                == Some(MonoNode::Fun(
                    root.fun,
                    root.variant.clone(),
                    root.inst.clone(),
                ))
        });
        if self.inst_opt.is_none() || analyzing_selected_root {
            // Collect function-typed parameters for behavioral predicate support.
            // This enables the Boogie backend to generate parameter variants.
            for param in target.func_env.get_parameters() {
                let param_ty = self.instantiate(&param.1);
                if let Type::Fun(fn_params, fn_results, _) = &param_ty {
                    let normalized_ty = self.normalize_fun_ty(param_ty.clone());
                    let fun_id = target
                        .func_env
                        .get_qualified_id()
                        .instantiate(self.inst_opt.clone().unwrap_or_default());
                    let info = FunParamInfo {
                        fun: fun_id.clone(),
                        param_sym: param.0,
                    };
                    self.info
                        .fun_param_infos
                        .entry(normalized_ty.clone())
                        .or_default()
                        .insert(info);
                    // Ensure the function type is also registered in fun_infos
                    self.info.fun_infos.entry(normalized_ty).or_default();
                    self.analyze_function_spec(&fun_id);
                    // Add the param and result types to done_types
                    self.add_type(fn_params.as_ref());
                    self.add_type(fn_results.as_ref());
                    // Register tuple type for results + mut ref outputs (for behavioral spec functions)
                    // This is needed when the ensures_of_results function returns a tuple.
                    // All types are dereferenced because behavioral specs work with values, not mutations.
                    let results_flat = fn_results.clone().flatten();
                    let mut_ref_outputs: Vec<Type> = fn_params
                        .clone()
                        .flatten()
                        .into_iter()
                        .filter(|ty| ty.is_mutable_reference())
                        .map(|ty| ty.skip_reference().clone())
                        .collect();
                    let all_outputs: Vec<Type> = results_flat
                        .into_iter()
                        .chain(mut_ref_outputs)
                        .map(|ty| ty.skip_reference().clone())
                        .collect();
                    if all_outputs.len() >= 2 {
                        self.info.tuple_inst.insert(all_outputs);
                    }
                }
            }
        }

        // Derive additional verification instantiations only during package-wide
        // analysis. A root-specific analysis must remain rooted at exactly one
        // verification procedure.
        if self.inst_opt.is_none() && self.selected_root.is_none() {
            let fun_type_params_arity = target.get_type_parameter_count();
            let usage_state = UsageProcessor::analyze(self.targets, target.func_env, target.data);

            // collect instantiations
            let mut all_insts = BTreeSet::new();
            for lhs_m in usage_state.accessed.all.iter() {
                let lhs_ty = lhs_m.to_type();
                for rhs_m in usage_state.accessed.all.iter() {
                    let rhs_ty = rhs_m.to_type();

                    // make sure these two types unify before trying to instantiate them
                    let adapter = TypeUnificationAdapter::new_pair(&lhs_ty, &rhs_ty, true, true);
                    if adapter
                        .unify(&mut NoUnificationContext, Variance::SpecVariance, false)
                        .is_none()
                    {
                        continue;
                    }

                    // find all instantiation combinations given by this unification
                    let fun_insts = TypeInstantiationDerivation::progressive_instantiation(
                        std::iter::once(&lhs_ty),
                        std::iter::once(&rhs_ty),
                        true,
                        false,
                        true,
                        false,
                        fun_type_params_arity,
                        true,
                        false,
                    );
                    all_insts.extend(fun_insts);
                }
            }

            // mark all the instantiated targets as todo
            for fun_inst in all_insts {
                self.todo_funs.push((
                    target.func_env.get_qualified_id(),
                    target.data.variant.clone(),
                    fun_inst,
                ));
            }
        }
    }

    fn analyze_bytecode(&mut self, target: &FunctionTarget<'_>, bc: &Bytecode) {
        use Bytecode::*;
        use Operation::*;
        // For monomorphization, we only need to analyze function calls, not `pack` or other
        // instructions because the types those are using are reflected in locals which are analyzed
        // elsewhere.
        match bc {
            Call(_, _, Invoke, srcs, _) => {
                if let Some(fun) = srcs.last() {
                    let fun_type =
                        self.normalize_fun_ty(self.instantiate(target.get_local_type(*fun)));
                    self.info.applied_fun_types.insert(fun_type);
                }
            },
            Call(_, _, Function(mid, fid, targs), ..)
            | Call(_, _, Closure(mid, fid, targs, ..), ..) => {
                let module_env = &self.env.get_module(*mid);
                let callee_env = module_env.get_function(*fid);
                let actuals = self.instantiate_vec(targs);

                // the type reflection functions are specially handled here
                if self.env.get_extlib_address() == *module_env.get_name().addr() {
                    let qualified_name = format!(
                        "{}::{}",
                        module_env.get_name().name().display(self.env.symbol_pool()),
                        callee_env.get_name().display(self.env.symbol_pool()),
                    );
                    if qualified_name == TYPE_NAME_MOVE || qualified_name == TYPE_INFO_MOVE {
                        self.add_type(&actuals[0]);
                    }
                }
                if self.env.get_stdlib_address() == *module_env.get_name().addr() {
                    let qualified_name = format!(
                        "{}::{}",
                        module_env.get_name().name().display(self.env.symbol_pool()),
                        callee_env.get_name().display(self.env.symbol_pool()),
                    );
                    if qualified_name == TYPE_NAME_GET_MOVE {
                        self.add_type(&actuals[0]);
                    }
                }

                if callee_env.is_native_or_intrinsic() && !actuals.is_empty() {
                    // Mark the associated module to be instantiated with the given actuals.
                    // This will instantiate all functions in the module with matching number
                    // of type parameters.
                    self.info
                        .native_inst
                        .entry(callee_env.module_env.get_id())
                        .or_default()
                        .insert(actuals.clone());
                    // Also record at function granularity: `native_inst` is keyed per
                    // module, so it can't answer "was role R called with K?".
                    self.info
                        .intrinsic_calls
                        .entry(mid.qualified(*fid))
                        .or_default()
                        .insert(actuals);
                } else if !callee_env.is_opaque() && !callee_env.is_struct_api() {
                    // This call needs to be inlined, with targs instantiated by self.inst_opt.
                    // Struct API wrappers are excluded: their call sites are translated to native
                    // ops (Pack, BorrowField, etc.) in stackless_bytecode_generator, so there is
                    // no independent bytecode target to schedule for monomorphization.
                    // Schedule for later processing if this instance has not been processed yet.
                    let entry = (mid.qualified(*fid), FunctionVariant::Baseline, actuals);
                    self.add_dependency(MonoNode::Fun(entry.0, entry.1.clone(), entry.2.clone()));
                    if !self.done_funs.contains(&entry) {
                        self.todo_funs.push(entry);
                    }
                }

                // Record closure construction under the type of the constructed closure.
                // Notice we strip abilities here as the prover does not consider them.
                if let Call(_, dests, Closure(_mid, _fid, _targs, mask), ..) = bc {
                    let fun_type =
                        self.normalize_fun_ty(self.instantiate(target.get_local_type(dests[0])));
                    let fun = mid.qualified_inst(*fid, self.instantiate_vec(targs));
                    self.analyze_function_spec(&fun);
                    self.info
                        .fun_infos
                        .entry(fun_type)
                        .or_default()
                        .insert(ClosureInfo { fun, mask: *mask });
                }
            },
            Call(_, _, WriteBack(_, edge), ..) => {
                // In very rare occasions, not all types used in the function can appear in
                // function parameters, locals, and return values. Types hidden in the write-back
                // chain of a hyper edge is one such case. Therefore, we need an extra processing
                // to collect types used in borrow edges.
                //
                // TODO(mengxu): need to revisit this once the modeling for dynamic borrow is done
                self.add_types_in_borrow_edge(edge)
            },
            Prop(_, _, exp) => self.analyze_exp(exp),
            SaveMem(_, _, mem) => {
                let mem = self.instantiate_mem(mem.to_owned());
                let struct_env = self.env.get_struct_qid(mem.to_qualified_id());
                self.add_struct(struct_env, &mem.inst);
            },
            Call(_, _, HavocGlobal(mid, sid, inst), ..) => {
                // Loop-header memory havocs may reference memory the function
                // does not otherwise touch (e.g. from a `modifies_of` frame
                // declaration); register it so its memory variable is declared.
                let mem = self.instantiate_mem(mid.qualified_inst(*sid, inst.to_owned()));
                let struct_env = self.env.get_struct_qid(mem.to_qualified_id());
                self.add_struct(struct_env, &mem.inst);
            },
            _ => {},
        }
    }

    /// Normalize a function type — see `Type::normalize_fun`. Kept as a
    /// method for call-site brevity.
    fn normalize_fun_ty(&self, ty: Type) -> Type {
        ty.normalize_fun()
    }

    fn instantiate(&self, ty: &Type) -> Type {
        if let Some(inst) = &self.inst_opt {
            ty.instantiate(inst)
        } else {
            ty.clone()
        }
    }

    fn instantiate_vec(&self, targs: &[Type]) -> Vec<Type> {
        if let Some(inst) = &self.inst_opt {
            Type::instantiate_slice(targs, inst)
        } else {
            targs.to_owned()
        }
    }

    /// When a closure value targeting `fun` is recorded, register the resource
    /// memory accessed by `fun`'s spec. Behavioral predicates (`aborts_of`,
    /// `ensures_of`, `result_of`) evaluate the target's spec, so its memory
    /// must be in `structs` for Boogie to emit the `_$memory` declarations —
    /// even when the closure is never called directly under the current
    /// verification filter.
    fn analyze_function_spec(&mut self, fun: &QualifiedInstId<FunId>) {
        if !self.done_function_specs.insert(fun.clone()) {
            return;
        }
        let fun_env = self.env.get_function(fun.to_qualified_id());
        let mems: Vec<_> = fun_env
            .get_spec_used_memory()
            .iter()
            .chain(fun_env.get_spec_old_memory().iter())
            .cloned()
            .collect();
        for mem in mems {
            let mem = mem.instantiate(&fun.inst);
            let struct_env = self.env.get_struct_qid(mem.to_qualified_id());
            self.add_struct(struct_env, &mem.inst);
        }

        let exps = {
            let spec = fun_env.get_spec();
            spec.conditions
                .iter()
                .flat_map(|cond| cond.all_exps().cloned())
                .chain(spec.proof_exps().into_iter().cloned())
                .collect::<Vec<_>>()
        };
        let saved_inst = self.inst_opt.replace(fun.inst.clone());
        for exp in exps {
            self.analyze_exp(&exp);
        }
        self.inst_opt = saved_inst;
    }

    fn instantiate_mem(&self, mem: QualifiedInstId<StructId>) -> QualifiedInstId<StructId> {
        if let Some(inst) = &self.inst_opt {
            mem.instantiate(inst)
        } else {
            mem
        }
    }

    // Expression and Spec Fun Analysis
    // --------------------------------

    fn analyze_spec_fun(&mut self, fun: QualifiedId<SpecFunId>) {
        let module_env = self.env.get_module(fun.module_id);
        let decl = module_env.get_spec_fun(fun.id);
        for Parameter(_, ty, _) in &decl.params {
            self.add_type_root(ty)
        }
        self.add_type_root(&decl.result_type);
        if let Some(exp) = &decl.body {
            self.analyze_exp(exp)
        }
    }

    fn add_move_equality_congruence_spec_fun(&mut self, root: QualifiedInstId<SpecFunId>) {
        let mut todo = vec![root];
        while let Some(id) = todo.pop() {
            if !self
                .info
                .move_equality_congruence_spec_funs
                .insert(id.clone())
            {
                continue;
            }
            if let Some(body) = self.env.get_spec_fun(id.to_qualified_id()).body.clone() {
                todo.extend(
                    body.called_spec_funs(self.env)
                        .into_iter()
                        .map(|callee| callee.instantiate(&id.inst)),
                );
            }
        }
    }

    fn analyze_exp(&mut self, exp: &ExpData) {
        exp.visit_post_order(&mut |e| {
            let node_id = e.node_id();
            self.add_type_root(&self.env.get_node_type(node_id));
            for ref ty in self.env.get_node_instantiation(node_id) {
                self.add_type_root(ty);
            }
            // Handle Closure operations in spec expressions
            if let ExpData::Call(node_id, ast::Operation::Closure(mid, fid, mask), _) = e {
                let inst = self.instantiate_vec(&self.env.get_node_instantiation(*node_id));
                let fun = mid.qualified_inst(*fid, inst.clone());
                let fun_type =
                    self.normalize_fun_ty(self.instantiate(&self.env.get_node_type(*node_id)));
                self.analyze_function_spec(&fun);
                self.info
                    .fun_infos
                    .entry(fun_type)
                    .or_default()
                    .insert(ClosureInfo {
                        fun: fun.clone(),
                        mask: *mask,
                    });
                // Schedule the closure target for monomorphization so the
                // Boogie backend emits a Baseline procedure declaration. The
                // apply procedure for the closure type calls this declaration.
                // Without this, behavioural predicates over a function that
                // appears only in specs (no bytecode-level closure pack) would
                // produce an undeclared-procedure call.
                if let Some(callee_env) = self.env.get_function_opt(fun.to_qualified_id()) {
                    if !callee_env.is_native_or_intrinsic()
                        && !callee_env.is_opaque()
                        && !callee_env.is_struct_api()
                    {
                        let entry = (fun.to_qualified_id(), FunctionVariant::Baseline, inst);
                        self.add_dependency(MonoNode::Fun(
                            entry.0,
                            entry.1.clone(),
                            entry.2.clone(),
                        ));
                        if !self.done_funs.contains(&entry) {
                            self.todo_funs.push(entry);
                        }
                    }
                }
            }
            if let ExpData::Call(_, ast::Operation::Behavior(..), args) = e {
                if let Some(target) = args.first() {
                    let fun_type = self.normalize_fun_ty(
                        self.instantiate(&self.env.get_node_type(target.node_id())),
                    );
                    self.info.behavioral_fun_types.insert(fun_type);
                }
            }
            if let ExpData::Invoke(_, target, _) = e {
                let fun_type = self
                    .normalize_fun_ty(self.instantiate(&self.env.get_node_type(target.node_id())));
                self.info.applied_fun_types.insert(fun_type);
            }
            if let ExpData::Call(node_id, ast::Operation::SpecFunction(mid, fid, _), _) = e {
                let actuals = self.instantiate_vec(&self.env.get_node_instantiation(*node_id));
                let module = self.env.get_module(*mid);
                let spec_fun = module.get_spec_fun(*fid);
                if self
                    .env
                    .spec_fun_call_needs_move_equality_congruence(*node_id, mid.qualified(*fid))
                {
                    self.add_move_equality_congruence_spec_fun(
                        mid.qualified_inst(*fid, actuals.clone()),
                    );
                }

                // the type reflection functions are specially handled here
                if self.env.get_extlib_address() == *module.get_name().addr() {
                    let qualified_name = format!(
                        "{}::{}",
                        module.get_name().name().display(self.env.symbol_pool()),
                        spec_fun.name.display(self.env.symbol_pool()),
                    );
                    if qualified_name == TYPE_NAME_SPEC
                        || qualified_name == TYPE_INFO_SPEC
                        || qualified_name == TYPE_SPEC_IS_STRUCT
                    {
                        self.add_type(&actuals[0]);
                    }
                }
                if self.env.get_stdlib_address() == *module.get_name().addr() {
                    let qualified_name = format!(
                        "{}::{}",
                        module.get_name().name().display(self.env.symbol_pool()),
                        spec_fun.name.display(self.env.symbol_pool()),
                    );
                    if qualified_name == TYPE_NAME_GET_SPEC {
                        self.add_type(&actuals[0]);
                    }
                }

                if spec_fun.is_native && !actuals.is_empty() {
                    // Add module to native modules
                    self.info
                        .native_inst
                        .entry(module.get_id())
                        .or_default()
                        .insert(actuals);
                } else {
                    let entry = (mid.qualified(*fid), actuals);
                    self.add_dependency(MonoNode::SpecFun(entry.0, entry.1.clone()));
                    // Only if this call has not been processed yet, queue it for future processing.
                    if !self.done_spec_funs.contains(&entry) {
                        self.todo_spec_funs.push(entry);
                    }
                }
            }
            true // keep going
        });
    }

    // Type Analysis
    // -------------

    fn add_type_root(&mut self, ty: &Type) {
        if let Some(inst) = &self.inst_opt {
            let ty = ty.instantiate(inst);
            self.add_node_type(ty.clone());
            self.add_type(&ty)
        } else {
            self.add_node_type(ty.clone());
            self.add_type(ty)
        }
    }

    fn add_type(&mut self, ty: &Type) {
        if !self.done_types.insert(ty.to_owned()) {
            return;
        }
        ty.visit(&mut |t| match t {
            Type::Fun(..) => {
                self.info
                    .fun_infos
                    .entry(self.normalize_fun_ty(t.clone()))
                    .or_default();
            },
            Type::Vector(et) => {
                self.info.vec_inst.insert(et.as_ref().clone());
            },
            Type::Tuple(elems) if elems.len() >= 2 => {
                // Only collect proper tuples, tuples of size 0 and 1 do not exist.
                // Strip references since Boogie tuple types work with values only.
                let elems: Vec<Type> = elems.iter().map(|ty| ty.skip_reference().clone()).collect();
                self.info.tuple_inst.insert(elems);
            },
            Type::Struct(mid, sid, targs) => {
                self.add_struct(self.env.get_module(*mid).into_struct(*sid), targs)
            },
            Type::TypeParameter(idx) => {
                self.info.type_params.insert(*idx);
            },
            _ => {},
        });
    }

    fn add_struct(&mut self, struct_: StructEnv<'_>, targs: &[Type]) {
        self.add_node_type(Type::Struct(
            struct_.module_env.get_id(),
            struct_.get_id(),
            targs.to_owned(),
        ));
        if struct_.is_intrinsic_of(INTRINSIC_TYPE_MAP) {
            self.info
                .table_inst
                .entry(struct_.get_qualified_id())
                .or_default()
                .insert((targs[0].clone(), targs[1].clone()));
        } else if struct_.is_intrinsic() && !targs.is_empty() {
            self.info
                .native_inst
                .entry(struct_.module_env.get_id())
                .or_default()
                .insert(targs.to_owned());
        } else {
            self.info
                .structs
                .entry(struct_.get_qualified_id())
                .or_default()
                .insert(targs.to_owned());
            if struct_.has_variants() {
                for variant in struct_.get_variants() {
                    for field in struct_.get_fields_of_variant(variant) {
                        let field_ty = field.get_type().instantiate(targs);
                        self.add_type(&field_ty);
                        self.check_struct_fun_field(&struct_, &field, &field_ty, targs);
                    }
                }
            } else {
                for field in struct_.get_fields() {
                    let field_ty = field.get_type().instantiate(targs);
                    self.add_type(&field_ty);
                    self.check_struct_fun_field(&struct_, &field, &field_ty, targs);
                }
            }
            // Ghost fields participate in the datatype constructor, so their
            // types must be monomorphized too — otherwise a type used only in
            // a ghost field never gets its own datatype emitted.
            for field in struct_.get_ghost_fields() {
                let field_ty = field.get_type().instantiate(targs);
                self.add_type(&field_ty);
                self.check_struct_fun_field(&struct_, &field, &field_ty, targs);
            }
        }
    }

    /// Check if a struct field has a storable function type and register it in
    /// `fun_struct_field_infos` if so.
    fn check_struct_fun_field(
        &mut self,
        struct_env: &StructEnv<'_>,
        field: &FieldEnv<'_>,
        field_ty: &Type,
        targs: &[Type],
    ) {
        if let Type::Fun(_, _, abilities) = field_ty {
            if abilities.has_store() {
                let normalized = self.normalize_fun_ty(field_ty.clone());
                // Normalize fun-type elements of the containing struct's
                // instantiation too: the constructor name is derived from
                // the boogie struct name, which drops fun abilities at every
                // nesting depth. Without normalizing here, two
                // ability-variant instantiations of the same wrapper (e.g.
                // `Option<|u64| has drop>` and
                // `Option<|u64| has drop + copy + store>`, directly or
                // nested as in `Option<Option<|u64| has drop>>`) would
                // produce two `StructFieldInfo` set entries mangling to one
                // datatype constructor.
                let normalized_targs: Vec<Type> = targs
                    .iter()
                    .map(|t| t.clone().normalize_nested_funs())
                    .collect();
                let info = StructFieldInfo {
                    struct_id: struct_env.get_qualified_id().instantiate(normalized_targs),
                    field_sym: field.get_name(),
                };
                self.info
                    .fun_struct_field_infos
                    .entry(normalized.clone())
                    .or_default()
                    .insert(info);
                for access in struct_env
                    .get_field_access_of()
                    .iter()
                    .filter(|access| access.fun_param == field.get_name())
                {
                    for memory in access.used_memory.iter().chain(&access.old_memory) {
                        self.add_type(&memory.clone().instantiate(targs).to_type());
                    }
                }
                // Ensure the function type is also registered in fun_infos
                self.info.fun_infos.entry(normalized).or_default();
            }
        }
    }

    // Utility functions
    // -----------------

    fn add_types_in_borrow_edge(&mut self, edge: &BorrowEdge) {
        match edge {
            BorrowEdge::Direct | BorrowEdge::Invoke | BorrowEdge::Index(_) => (),
            BorrowEdge::Field(qid, _, _) => {
                self.add_type_root(&qid.to_type());
            },
            BorrowEdge::Hyper(edges) => {
                for item in edges {
                    self.add_types_in_borrow_edge(item);
                }
            },
        }
    }

    fn add_dependency(&mut self, dependency: MonoNode) {
        if let Some(node) = &self.current_node {
            self.node_deps
                .entry(node.clone())
                .or_default()
                .insert(dependency);
        }
    }

    fn add_node_type(&mut self, ty: Type) {
        if let Some(node) = &self.current_node {
            self.node_types.entry(node.clone()).or_default().insert(ty);
        }
    }

    fn compute_root_slices(&self) -> BTreeMap<VerificationRoot, MonoSlice> {
        self.selected_root
            .iter()
            .map(|root| {
                let root_node = MonoNode::Fun(root.fun, root.variant.clone(), root.inst.clone());
                let mut todo = vec![root_node];
                let mut seen_nodes = BTreeSet::new();
                let mut root_types = BTreeSet::new();
                while let Some(node) = todo.pop() {
                    if !seen_nodes.insert(node.clone()) {
                        continue;
                    }
                    if let Some(types) = self.node_types.get(&node) {
                        root_types.extend(types.iter().cloned());
                    }
                    if let Some(dependencies) = self.node_deps.get(&node) {
                        todo.extend(dependencies.iter().cloned());
                    }
                }

                let mut slice = MonoSlice::default();
                let mut seen_types = BTreeSet::new();
                for ty in root_types {
                    self.collect_slice_structs(&ty, &mut seen_types, &mut slice.structs);
                }
                // Only retain instances for which the package-wide analysis emits
                // a concrete datatype and generated validity/equality predicates.
                slice.structs.retain(|qid, insts| {
                    if let Some(global_insts) = self.info.structs.get(qid) {
                        insts.retain(|inst| global_insts.contains(inst));
                        !insts.is_empty()
                    } else {
                        false
                    }
                });
                (root.clone(), slice)
            })
            .collect()
    }

    fn collect_slice_structs(
        &self,
        ty: &Type,
        seen: &mut BTreeSet<Type>,
        structs: &mut BTreeMap<QualifiedId<StructId>, BTreeSet<Vec<Type>>>,
    ) {
        ty.visit(&mut |nested| {
            if !seen.insert(nested.clone()) {
                return;
            }
            if let Type::Struct(mid, sid, inst) = nested {
                let struct_env = self.env.get_struct(mid.qualified(*sid));
                if !struct_env.is_intrinsic() {
                    structs
                        .entry(mid.qualified(*sid))
                        .or_default()
                        .insert(inst.clone());
                    if struct_env.has_variants() {
                        for variant in struct_env.get_variants() {
                            for field in struct_env.get_fields_of_variant(variant) {
                                self.collect_slice_structs(
                                    &field.get_type().instantiate(inst),
                                    seen,
                                    structs,
                                );
                            }
                        }
                    } else {
                        for field in struct_env.get_fields() {
                            self.collect_slice_structs(
                                &field.get_type().instantiate(inst),
                                seen,
                                structs,
                            );
                        }
                    }
                    for field in struct_env.get_ghost_fields() {
                        self.collect_slice_structs(
                            &field.get_type().instantiate(inst),
                            seen,
                            structs,
                        );
                    }
                }
            }
        });
    }
}
