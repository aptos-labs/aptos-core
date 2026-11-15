// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Which memories code outside the program cannot write.
//!
//! Code a function value runs may call any function the program lets it reach, but obtains
//! (receives or keeps) no signer for a fixed system address (trusted fact, see
//! `ProverOptions::trust_system_address_signers`); signers the program mints and consumes
//! internally are covered by the analysis of where they go. So a memory is
//! *system-address-bound* when every write of it in the program -- `move_to`,
//! `borrow_global_mut`, `move_from`, in any instantiation -- is at a system address, and every
//! path from code unknown code can run to the write passes a check that a signer that code
//! supplied has a system address:
//!
//! - the address is a constant system address, the address of a signer the writing function
//!   received and checked before the write on every path, or that of a signer it minted for a
//!   constant system address;
//! - the writing function is not in `U` (below), or it is in `U` but not `U*` and a check
//!   precedes the write on every path.
//!
//! A check is a call to one of `CHECKS` on a signer the function received (or its address), a
//! branch on its address equal to a system address (`==`, `IS_SYSTEM`), or a call to a function
//! of the program that makes such a check on that parameter on every path to its return.
//!
//! `U` grows from the functions unknown code can enter -- `public` ones, `friend` ones of a
//! module with a friend outside the program, and every closure target -- through each call no
//! check precedes. An `entry` function that is not public is not a root: a transaction may call
//! it, code run by a callback cannot. A check on a signer the program minted, or got from a
//! call, proves nothing, so a call handed such a signer, the target of a closure capturing a
//! signer, and any function a signer-passing invocation or dispatch native may run, start
//! `U*`, where checks count for nothing.
//!
//! The program itself must not hand unknown code a dangerous value: one that may hold a signer,
//! a capability a signer can be produced from, or a function value (`sensitive`), and is not
//! only received from the caller or hash-derived. It leaks when it reaches unknown code:
//! returned past no check by a function unknown code enters (or stored through a mutable
//! reference that function received), handed to a function value of unknown target or a
//! dispatch native, or held by a closure doing either; a leak means nothing qualifies. A
//! closure holds what it captured and what its target returns from that. Provenance follows
//! calls through per-function summaries, and invocations of closures the program made through
//! their targets' summaries; what invoking a received function value returns, and what a
//! callee hands unknown code, are resolved by each caller from the arguments it passes. What
//! the analysis cannot resolve is dangerous. A signer produced from a received capability was
//! the caller's already; hash-derived addresses (`NON_SYSTEM_PRODUCERS`) are never system
//! addresses; the returns of `TRUSTED_SIGNER_SOURCES` are exempt, each by the trusted fact or a
//! hash-derived address.
//!
//! Unknown is the default. A function value is of known target only when all it carries are
//! closure targets and what a closure captures for a known address; a constant holds every
//! address in it; a generic struct keeps what all its fields hold, since a generic function
//! takes a field out as a type parameter, where nothing marks it as of unknown origin; and a
//! value with no provenance at all, consumed as authority (by a minter, a capability's address,
//! a signer producer or an invocation), is of unknown origin (`Prov::unrepresented`).
//!
//! Only the defining module writes a resource (the bytecode verifier rejects global operations
//! on other modules' structs, also after inlining), so the scan covers every writer. A module
//! with a native may access its storage without that check, so it never qualifies. Computed
//! once per environment from the compiled bytecode.

use crate::{
    function_target::FunctionData,
    stackless_bytecode::{Bytecode, Constant, Operation},
    stackless_bytecode_generator::StacklessBytecodeGenerator,
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use move_binary_format::access::ModuleAccess;
use move_core_types::{ability::Ability, account_address::AccountAddress};
use move_model::{
    ast::{Address, TempIndex},
    model::{FunId, FunctionEnv, GlobalEnv, ModuleId, QualifiedId, StructId, Visibility},
    ty::{PrimitiveType, Type},
};
use std::collections::{BTreeMap, BTreeSet};

/// Whether `system_address_bound` may hold for any memory (the prover option).
pub struct TrustSystemAddressSigners(pub bool);

/// The system-address-bound structs of the environment, computed on first use.
struct SystemAddressBoundStructs(BTreeSet<QualifiedId<StructId>>);

/// Sets whether `system_address_bound` may hold, i.e. whether its trusted fact is assumed.
pub fn set_trust_system_address_signers(env: &GlobalEnv, trust: bool) {
    env.set_extension(TrustSystemAddressSigners(trust));
}

/// Whether no code outside the program can change memory `mem` (see the module doc). False
/// unless `set_trust_system_address_signers` enabled it, for spec variables and for the world
/// memory.
pub fn system_address_bound(env: &GlobalEnv, mem: &QualifiedId<StructId>) -> bool {
    if !env
        .get_extension::<TrustSystemAddressSigners>()
        .is_some_and(|trust| trust.0)
    {
        return false;
    }
    if env.is_world_memory(mem) || env.get_struct_qid(*mem).is_ghost_memory() {
        return false;
    }
    if !env.has_extension::<SystemAddressBoundStructs>() {
        env.set_extension(SystemAddressBoundStructs(system_address_bound_structs(env)));
    }
    env.get_extension::<SystemAddressBoundStructs>()
        .expect("extension set above")
        .0
        .contains(mem)
}

/// The fixed system addresses: the VM (@0x0), the framework-reserved @0x1..@0xa
/// (`system_addresses::is_framework_reserved_address`) and @core_resources.
fn is_system_address(addr: &Address) -> bool {
    let Address::Numerical(addr) = addr else {
        return false;
    };
    let bytes = addr.into_bytes();
    let (high, low) = bytes.split_at(AccountAddress::LENGTH - 4);
    let low = u32::from_be_bytes(low.try_into().expect("four bytes"));
    high.iter().all(|b| *b == 0) && (low <= 0xA || low == 0xA550C18)
}

/// The functions that abort unless their argument -- a signer, or a signer's address -- is a
/// system address: every such assertion in `system_addresses` and `features`.
const CHECKS: &[&str] = &[
    "system_addresses::assert_aptos_framework",
    "system_addresses::assert_core_resource",
    "system_addresses::assert_core_resource_address",
    "system_addresses::assert_framework_reserved",
    "system_addresses::assert_framework_reserved_address",
    "system_addresses::assert_vm",
    "features::ensure_framework_signer",
];

/// The functions returning whether their argument -- a signer, or an address -- is a system
/// address.
const IS_SYSTEM: &[&str] = &[
    "system_addresses::is_aptos_framework_address",
    "system_addresses::is_core_resource_address",
    "system_addresses::is_framework_reserved_address",
    "system_addresses::is_reserved_address",
    "system_addresses::is_vm",
    "system_addresses::is_vm_address",
];

/// The functions returning a signer for their address argument.
const MINTERS: &[&str] = &[
    "create_signer::create_signer",
    "account::create_signer_for_test",
    "account::create_framework_reserved_account",
];

/// The functions returning a signer for the capability or reference they receive (argument 0).
/// `permissioned_signer::signer_from_permissioned_handle` is not one: it always aborts.
const SIGNER_PRODUCERS: &[&str] = &[
    "account::create_signer_with_capability",
    "object::generate_signer",
    "object::generate_signer_for_extending",
];

/// The natives that only move, borrow or count the values they are handed, of the standard
/// containers and storage slots: what they return holds only what their operands held.
const STRUCTURAL_NATIVES: &[&str] = &[
    "vector::empty",
    "vector::length",
    "vector::borrow",
    "vector::borrow_mut",
    "vector::push_back",
    "vector::pop_back",
    "vector::destroy_empty",
    "vector::swap",
    "vector::move_range",
    "mem::swap",
    "table::add_box",
    "table::borrow_box",
    "table::borrow_box_mut",
    "table::contains_box",
    "table::remove_box",
    "table::destroy_empty_box",
    "table::drop_unchecked_box",
    "storage_slot::borrow_storage_slot_resource",
    "storage_slot::borrow_storage_slot_resource_mut",
    "aggregator_v2::create_aggregator",
    "aggregator_v2::create_unbounded_aggregator",
    "aggregator_v2::read",
    "aggregator_v2::snapshot",
    "aggregator_v2::create_snapshot",
    "aggregator_v2::read_snapshot",
    "aggregator_v2::try_add",
    "aggregator_v2::try_sub",
    "aggregator_v2::derive_string_concat",
];

/// Move functions that only move the value they are handed into or out of a handle: what they
/// return holds only what their operands held. Callers judge their results; their own bodies
/// lose the provenance through storage, so they are not judged as roots. A `StorageSlot` is
/// linear, `new` is its only constructor, and `StorageSlotResource` at its address is written
/// only by `new` and removed only by `destroy`.
const STRUCTURAL_FUNCTIONS: &[&str] = &["storage_slot::new", "storage_slot::destroy"];

/// The functions keeping one of their arguments (the position given) for whoever later takes it
/// out: handing them a dangerous value is a leak. `voting::resolve` hands a proposal's execution
/// content back to whoever resolves it; `pack` is the only constructor of an `Any`, whose
/// `unpack` rebuilds the packed value (`from_bcs::from_bytes` is a friend of these modules only).
const CONTENT_SINKS: &[(&str, usize)] = &[
    ("voting::create_proposal", 2),
    ("voting::create_proposal_v2", 2),
    ("any::pack", 0),
    ("copyable_any::pack", 0),
];

/// The functions not judged as roots for what they return, each because something else judges
/// it, and whose result callers see as `CHECKED`:
/// - `voting::resolve` and `any`/`copyable_any` `unpack` return a value only `CONTENT_SINKS`
///   put there, which they check (or which unknown code put there, its own);
/// - `reflect::resolve` returns a function value of a public function capturing nothing (the VM
///   resolves only public functions, `native_functions.rs` `resolve_function`, and builds the
///   closure with no captured arguments, `reflect.rs`).
const JUDGED_ELSEWHERE: &[&str] = &[
    "voting::resolve",
    "any::unpack",
    "copyable_any::unpack",
    "reflect::resolve",
];

/// The functions whose results hold only addresses hashed with a domain separator (sha3-256 of
/// the creator's address or a transaction-unique value), which are never system addresses.
const NON_SYSTEM_PRODUCERS: &[&str] = &[
    "account::create_resource_address",
    "object::create_guid_object_address",
    "object::create_object_address",
    "object::create_object_from_guid",
    "object::create_user_derived_object_address",
    "transaction_context::generate_auid_address",
];

/// The public functions whose returned signer the scan cannot show is not for a system address.
/// Each entry rests on the trusted fact or on hash-derived addresses never being system ones.
const TRUSTED_SIGNER_SOURCES: &[&str] = &[
    // An approved governance proposal's signer: the trusted fact's stated exception.
    "aptos_governance::resolve",
    "aptos_governance::resolve_multi_step_proposal",
    // Only for an address whose `Account` offers its signer capability to the caller; only that
    // address's own signer makes such an offer (`offer_signer_capability`), which for a system
    // address means a governance action.
    "account::create_authorized_signer",
    // A signer for the calling module's own address; every module at a system address is
    // framework code, none of which calls it.
    "init::internal_maybe_initialize",
    // `VestingContract.signer_cap` is only built by `account::create_resource_account`, whose
    // address is hash-derived.
    "vesting::get_vesting_account_signer",
    // `ManagingRefs.extend_ref` only comes from `object::create_named_object`, whose address is
    // hash-derived.
    "object_code_deployment::get_code_object_signer",
    // `Container.store` is only filled with capabilities `account::create_resource_account`
    // returns, whose address is hash-derived.
    "resource_account::retrieve_resource_account_cap",
];

/// The `0x1` function `fun` is, as `module::name`, if it is one.
fn framework_name(fun: &FunctionEnv) -> Option<String> {
    let Address::Numerical(addr) = fun.module_env.self_address() else {
        return None;
    };
    (*addr == AccountAddress::ONE).then(|| fun.get_full_name_str())
}

/// Whether a value of type `ty` may hold a signer whatever its type parameters are: as a signer,
/// in a struct field (with the type arguments substituted), behind a struct of a module declaring
/// natives (a table's values), or captured by a function value without `store`. A type parameter
/// holds none here: the call instantiating it is judged for what it passes.
fn holds_signer(env: &GlobalEnv, ty: &Type, caps: &SensitiveTypes) -> bool {
    match ty {
        Type::Primitive(PrimitiveType::Signer) => true,
        Type::Primitive(_) | Type::TypeParameter(_) => false,
        Type::Fun(_, _, abilities) => !abilities.has_ability(Ability::Store),
        Type::Reference(_, ty) | Type::Vector(ty) => holds_signer(env, ty, caps),
        Type::Tuple(tys) => tys.iter().any(|t| holds_signer(env, t, caps)),
        Type::Struct(mid, sid, args) => {
            (caps.native_modules.contains(mid) && args.iter().any(|t| holds_signer(env, t, caps)))
                || env
                    .get_struct(mid.qualified(*sid))
                    .get_fields()
                    .any(|f| holds_signer(env, &f.get_type().instantiate(args), caps))
        },
        _ => true,
    }
}

fn has_signer(ty: &Type) -> bool {
    let mut found = false;
    ty.visit(&mut |t| found |= t.is_signer());
    found
}

/// Whether a value of type `ty` may hold a signer, or, with `address`, also an address: through
/// struct fields, type parameters and what a function value captures.
fn may_hold(env: &GlobalEnv, ty: &Type, address: bool) -> bool {
    fn holds(env: &GlobalEnv, ty: &Type, address: bool, type_params: bool) -> bool {
        match ty {
            Type::Primitive(PrimitiveType::Signer) => true,
            Type::Primitive(PrimitiveType::Address) => address,
            Type::Primitive(_) => false,
            Type::TypeParameter(_) => type_params,
            // A closure with `store` captures only values with `store`, never a signer.
            Type::Fun(_, _, abilities) => address || !abilities.has_ability(Ability::Store),
            Type::Reference(_, ty) | Type::Vector(ty) => holds(env, ty, address, type_params),
            Type::Tuple(tys) => tys.iter().any(|t| holds(env, t, address, type_params)),
            Type::Struct(mid, sid, args) => {
                args.iter().any(|t| holds(env, t, address, type_params))
                    || env
                        .get_struct(mid.qualified(*sid))
                        .get_fields()
                        .any(|f| holds(env, &f.get_type(), address, false))
            },
            _ => true,
        }
    }
    holds(env, ty, address, true)
}

/// What a temporary holds, as far as signers and addresses go, on every path.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Value {
    /// Not yet assigned on any path.
    Unassigned,
    /// A constant system address.
    SystemAddress,
    /// The value the function received for this parameter: a signer, or an immutable reference to
    /// one. A `&mut signer` parameter may be overwritten, so it starts as `Other`.
    Param(TempIndex),
    /// The address of that parameter's signer.
    ParamAddress(TempIndex),
    /// Whether that address is a system address.
    IsSystem(TempIndex),
    /// Whether it is not.
    IsNotSystem(TempIndex),
    /// A signer minted for a constant system address, or a reference to one.
    MintedSystem,
    /// The address of such a signer.
    MintedSystemAddress,
    /// An address read out of a capability, as safe as the capability.
    CapAddress,
    /// Anything else.
    Other,
}

impl Value {
    fn join(self, other: Value) -> Value {
        match (self, other) {
            (Value::Unassigned, v) | (v, Value::Unassigned) => v,
            (a, b) if a == b => a,
            _ => Value::Other,
        }
    }

    fn address(self) -> Value {
        match self {
            Value::Param(p) | Value::ParamAddress(p) => Value::ParamAddress(p),
            Value::MintedSystem | Value::MintedSystemAddress => Value::MintedSystemAddress,
            _ => Value::Other,
        }
    }

    /// The parameter whose signer this is, or whose address.
    fn param(self) -> Option<TempIndex> {
        match self {
            Value::Param(p) | Value::ParamAddress(p) => Some(p),
            _ => None,
        }
    }

    /// What a caller learns of a returned value.
    fn summarized(self) -> Value {
        match self {
            Value::Param(_)
            | Value::Unassigned
            | Value::ParamAddress(_)
            | Value::SystemAddress
            | Value::MintedSystem
            | Value::MintedSystemAddress
            | Value::CapAddress => self,
            _ => Value::Other,
        }
    }

    /// A summarized returned value, given the values of the call's arguments.
    fn instantiate(self, args: &[Value]) -> Value {
        match self {
            Value::Param(p) => args.get(p).copied().unwrap_or(Value::Other),
            Value::ParamAddress(p) => args.get(p).map_or(Value::Other, |v| v.address()),
            v => v,
        }
    }
}

/// A hash-derived address, or a signer, capability or reference for one.
const NON_SYSTEM: u8 = 1;
/// Read from storage, returned by a native or by unknown code, or computed.
const OTHER: u8 = 2;
/// A constant system address.
const SYSTEM_ADDRESS: u8 = 4;
/// A signer minted for a constant system address.
const MINTED_SYSTEM: u8 = 8;
/// A signer minted for an address that may be a system address.
const MAYBE_SYSTEM: u8 = 16;
/// Checked where it entered (`JUDGED_ELSEWHERE`): not dangerous, but of unknown content, so a
/// function value of it has an unknown target.
const CHECKED: u8 = 32;
/// A mutable reference into a capability's fields: whoever holds it can redirect the capability,
/// so handing it out is handing out authority to mint for any address.
const CAP_FIELD_REF: u8 = 64;

/// What unknown code hands back: of unknown origin, and possibly a signer for a system address or
/// a function value of unknown target.
const UNKNOWN_CODE: u8 = OTHER | MAYBE_SYSTEM;

/// Where the addresses, signers, capabilities and function values a value may hold come from, on
/// some path. The join is the union. A sensitive value (`sensitive`) is dangerous unless it is
/// only received (`params`, `produced`, `invoked`: resolved by each caller) or hash-derived
/// (`NON_SYSTEM`): from storage, a native, unknown code or a computation (`OTHER`), for a
/// constant system address, or minted for any address the program cannot show is not one.
///
/// For the value fed to a minter or a `SIGNER_PRODUCERS` function, `OTHER` and
/// `SYSTEM_ADDRESS` make the signer `MAYBE_SYSTEM` (`MINTED_SYSTEM` for a constant).
#[derive(Clone, Default, PartialEq, Eq, Debug)]
struct Prov {
    /// The parameters it may derive from.
    params: BTreeSet<TempIndex>,
    /// The parameters whose capability may have produced a signer it holds.
    produced: BTreeSet<TempIndex>,
    origins: u8,
    /// The closure targets of the function values it may hold.
    targets: BTreeSet<QualifiedId<FunId>>,
    /// The parameters whose function value, invoked, may have returned or stored it: resolved by
    /// each caller from the argument it passes.
    invoked: BTreeSet<TempIndex>,
    /// The address parameters a capability it holds was packed from: a mint, judged by each
    /// caller from the address it passes (`minted`); at a root, the address is unknown code's.
    minted_from: BTreeSet<TempIndex>,
}

impl Prov {
    fn origin(origins: u8) -> Prov {
        Prov {
            origins,
            ..Prov::default()
        }
    }

    fn param(p: TempIndex) -> Prov {
        Prov {
            params: BTreeSet::from([p]),
            ..Prov::default()
        }
    }

    fn join(&mut self, other: &Prov) {
        self.params.extend(other.params.iter().copied());
        self.produced.extend(other.produced.iter().copied());
        self.origins |= other.origins;
        self.targets.extend(other.targets.iter().copied());
        self.invoked.extend(other.invoked.iter().copied());
        self.minted_from.extend(other.minted_from.iter().copied());
    }

    /// Whether nothing says where the value came from. Once the summaries are complete
    /// (`Context::strict`), this is a value the analysis does not represent, and whatever
    /// consumes it as authority (a minter, a signer producer, an invocation) takes it for a
    /// value of unknown origin.
    fn unrepresented(&self) -> bool {
        self.origins == 0
            && self.params.is_empty()
            && self.produced.is_empty()
            && self.invoked.is_empty()
            && self.minted_from.is_empty()
    }

    fn dangerous(&self) -> bool {
        self.origins & !(NON_SYSTEM | CHECKED) != 0
    }

    /// Whether a root may not hand this out: dangerous, or a capability packed from one of the
    /// root's own address parameters, which unknown code chooses.
    fn dangerous_at_root(&self) -> bool {
        self.dangerous() || !self.minted_from.is_empty()
    }

    /// A capability packed from an address with this provenance (a mint judged without the
    /// address's value): hash-derived stays so; a constant system address makes it minted for
    /// one; an unknown origin, or what a function value returned, makes it possibly one; an
    /// address parameter is left to the caller (`minted_from`), and a received signer's or
    /// capability's address keeps its source.
    fn minted(&self, strict: bool) -> Prov {
        let mut origins = self.origins & NON_SYSTEM;
        if self.origins & (SYSTEM_ADDRESS | MINTED_SYSTEM) != 0 {
            origins |= MINTED_SYSTEM;
        }
        if self.origins & (OTHER | MAYBE_SYSTEM | CHECKED) != 0
            || !self.invoked.is_empty()
            || (strict && self.unrepresented())
        {
            origins |= MAYBE_SYSTEM;
        }
        Prov {
            produced: self.produced.clone(),
            origins,
            minted_from: self.params.union(&self.minted_from).copied().collect(),
            ..Prov::default()
        }
    }

    /// A signer minted for an address with this provenance and value.
    fn mint(&self, addr: Value, strict: bool) -> Prov {
        match addr {
            // The signer of a capability's address is as safe as the capability.
            Value::CapAddress => self.produce(strict),
            Value::SystemAddress | Value::MintedSystemAddress => Prov::origin(MINTED_SYSTEM),
            // The received signer minted again, unless something dangerous was stored into it.
            Value::ParamAddress(p) => {
                let mut prov = Prov::param(p);
                prov.origins = self.origins & (MINTED_SYSTEM | MAYBE_SYSTEM);
                prov.minted_from = self.minted_from.clone();
                prov
            },
            _ => {
                let mut origins = self.origins & NON_SYSTEM;
                if self.origins & (SYSTEM_ADDRESS | MINTED_SYSTEM) != 0 {
                    origins |= MINTED_SYSTEM;
                }
                if self.origins & (OTHER | MAYBE_SYSTEM | CHECKED) != 0
                    || !self.params.is_empty()
                    || !self.invoked.is_empty()
                    || (strict && self.unrepresented())
                {
                    origins |= MAYBE_SYSTEM;
                }
                Prov {
                    produced: self.produced.clone(),
                    origins,
                    // An address taken out of a capability packed from a parameter is still that
                    // parameter's: the caller judges it.
                    minted_from: self.minted_from.clone(),
                    ..Prov::default()
                }
            },
        }
    }

    /// A signer produced from a capability with this provenance.
    fn produce(&self, strict: bool) -> Prov {
        let mut origins = self.origins & NON_SYSTEM;
        if self.origins & !NON_SYSTEM != 0
            || !self.invoked.is_empty()
            || (strict && self.unrepresented())
        {
            origins |= MAYBE_SYSTEM;
        }
        Prov {
            produced: self.params.union(&self.produced).copied().collect(),
            origins,
            minted_from: self.minted_from.clone(),
            ..Prov::default()
        }
    }

    /// A callee's summarized provenance, given the provenance of the call's arguments, leaving
    /// out what invoking a received function value returned (`Context::instantiate`). `strict`
    /// says, per parameter, whether an argument nothing is known of counts as of unknown origin
    /// (`unrepresented`).
    fn instantiate(&self, args: &[Prov], strict: &dyn Fn(usize) -> bool) -> Prov {
        let mut out = Prov::origin(self.origins);
        out.targets = self.targets.clone();
        for arg in self.params.iter().filter_map(|p| args.get(*p)) {
            out.join(arg);
        }
        for (p, arg) in self
            .produced
            .iter()
            .filter_map(|p| Some((*p, args.get(*p)?)))
        {
            out.join(&arg.produce(strict(p)));
        }
        for (p, arg) in self
            .minted_from
            .iter()
            .filter_map(|p| Some((*p, args.get(*p)?)))
        {
            out.join(&arg.minted(strict(p)));
        }
        out
    }
}

/// The state before an instruction: what each temporary holds, and the parameters whose signer
/// was checked on every path to it.
#[derive(Clone, PartialEq, Eq)]
struct State {
    values: Vec<Value>,
    provs: Vec<Prov>,
    checked: BTreeSet<TempIndex>,
}

impl State {
    fn join(&self, other: &State) -> State {
        State {
            values: self
                .values
                .iter()
                .zip(&other.values)
                .map(|(a, b)| a.join(*b))
                .collect(),
            provs: self
                .provs
                .iter()
                .zip(&other.provs)
                .map(|(a, b)| {
                    let mut a = a.clone();
                    a.join(b);
                    a
                })
                .collect(),
            checked: self.checked.intersection(&other.checked).copied().collect(),
        }
    }

    fn gated(&self) -> bool {
        !self.checked.is_empty()
    }
}

/// A write of a struct: whether its address is a system address, and whether a check precedes
/// it.
struct Write {
    mem: QualifiedId<StructId>,
    /// A `move_to`, which publishes the resource.
    publishes: bool,
    system_address: bool,
    gated: bool,
}

/// A call: whether a check precedes it, and whether it is handed a signer that is not one the
/// caller received.
struct Call {
    callee: QualifiedId<FunId>,
    gated: bool,
    unreceived_signer: bool,
}

/// What a function's callers learn of it.
#[derive(Clone, Default, PartialEq, Eq)]
struct Summary {
    /// Per return position, the value on every path to a return and what it may hold; `None`
    /// without a reachable return.
    returns: Option<(Vec<Value>, Vec<Prov>)>,
    /// What it may store through the mutable references it receives.
    stored: Prov,
    /// What may reach unknown code, as a sensitive value.
    sinks: Prov,
    /// The same for values sensitive only if a type parameter is instantiated so, which the
    /// instantiating call judges.
    generic_sinks: Prov,
    /// What may reach unknown code as a value that may hold a signer (`holds_signer`). A signer
    /// a function received is its caller's, and each caller resolves it from what it passes;
    /// where the caller may be the VM or a governance script (`outside_callable`), it may be a
    /// signer for a system address, which unknown code must not get.
    signer_sinks: Prov,
}

impl Summary {
    /// Takes in the summary a round computed. A returned value only ascends `Unassigned`, a
    /// value, `Other` (`Value::join`), and the rest only grows within finite sets.
    fn advance(&mut self, now: &Summary) {
        self.returns = match (self.returns.take(), &now.returns) {
            (Some((values, provs)), Some((now_values, now_provs))) => Some((
                values
                    .into_iter()
                    .zip(now_values)
                    .map(|(v, now)| v.join(*now))
                    .collect(),
                provs
                    .into_iter()
                    .zip(now_provs)
                    .map(|(mut p, now)| {
                        p.join(now);
                        p
                    })
                    .collect(),
            )),
            (prev, now) => prev.or_else(|| now.clone()),
        };
        self.stored.join(&now.stored);
        self.sinks.join(&now.sinks);
        self.generic_sinks.join(&now.generic_sinks);
        self.signer_sinks.join(&now.signer_sinks);
    }
}

#[derive(Default)]
struct Facts {
    writes: Vec<Write>,
    calls: Vec<Call>,
    /// Closure targets, their parameters left open, and whether the closure captures a signer.
    closures: Vec<(QualifiedId<FunId>, Vec<Type>, bool)>,
    /// Parameter types of function values invoked with a signer.
    signer_invokes: Vec<Vec<Type>>,
    /// The parameters checked on every path to a return, `None` without a reachable return.
    checked_at_return: Option<BTreeSet<TempIndex>>,
    summary: Summary,
    /// Whether a return past no check, or a store through a mutable reference parameter, may
    /// hand its caller a dangerous value.
    hands_out: bool,
    /// Whether it may hand unknown code a dangerous value.
    leaks: bool,
}

impl Facts {
    /// Records that a sensitive value with provenance `prov` reaches unknown code: a leak if it
    /// is dangerous, and what callers resolve. With `generic`, it is sensitive only through a
    /// type parameter. With `signer`, it may hold a signer.
    fn reach_unknown(&mut self, prov: &Prov, generic: bool, signer: bool) {
        if generic {
            self.summary.generic_sinks.join(prov);
        } else {
            self.leaks |= prov.dangerous();
            self.summary.sinks.join(prov);
            if signer {
                self.summary.signer_sinks.join(prov);
            }
        }
    }
}

/// What counts as a check: the functions checking parameters on every path to their return,
/// with those parameters, and the structs published only at system addresses, whose existence
/// at an address shows it is one.
#[derive(Clone, Default, PartialEq, Eq)]
struct Checks {
    functions: BTreeMap<QualifiedId<FunId>, BTreeSet<TempIndex>>,
    capabilities: BTreeSet<QualifiedId<StructId>>,
}

/// What a round knows of the program while analyzing one function.
struct Context<'a> {
    env: &'a GlobalEnv,
    types: &'a [Type],
    /// Per temporary, whether it may hold an address or a signer.
    bears: Vec<bool>,
    checks: &'a Checks,
    summaries: &'a BTreeMap<QualifiedId<FunId>, Summary>,
    /// The functions with a summary: every one with code.
    analyzed: &'a BTreeSet<QualifiedId<FunId>>,
    /// What `sensitive` needs of the program.
    cap_types: &'a SensitiveTypes,
    /// Per temporary, whether it is sensitive (`sensitive`) whatever its type parameters.
    sens: Vec<bool>,
    /// Per temporary, whether it is sensitive only through a type parameter.
    gsens: Vec<bool>,
    /// Per temporary, whether it may hold a function value.
    funs: Vec<bool>,
    /// Per temporary, whether a write through a reference may change it: a reference, or a
    /// local borrowed mutably.
    reachable: Vec<bool>,
    /// Per temporary, whether it may hold a capability (concretely: only code of the
    /// capability's module writes its fields).
    cap_holders: Vec<bool>,
    /// Whether a mutable reference parameter may reach a capability.
    cap_ref_param: bool,
    /// Whether the summaries are complete (see `system_address_bound_structs`): a value with no
    /// provenance is then one the analysis does not represent, not one a later round resolves.
    strict: bool,
}

impl Context<'_> {
    fn writes_through(&self, temps: &[TempIndex]) -> bool {
        temps
            .iter()
            .any(|t| self.types[*t].is_mutable_reference() && self.bears[*t])
    }

    /// A write through a mutable reference: anything it reaches may now hold a sensitive value
    /// of provenance `sensitive`, and a non-sensitive value, or a capability (whose provenance
    /// covers its address field), may also hold `other`.
    fn store(
        &self,
        state: &mut State,
        facts: Option<&mut Facts>,
        sensitive: &Prov,
        other: &Prov,
        cap_address: &Prov,
    ) {
        for (t, p) in state.provs.iter_mut().enumerate() {
            if self.bears[t] && self.reachable[t] {
                p.join(sensitive);
                if self.sensitivity(t).is_none() {
                    p.join(other);
                } else if self.cap_holders[t] {
                    p.join(cap_address);
                }
            }
        }
        if let Some(facts) = facts {
            facts.summary.stored.join(sensitive);
            // An address written into a capability the caller holds changes whose signer it
            // produces: part of what this function stores through its mutable references.
            if self.cap_ref_param {
                facts.summary.stored.join(cap_address);
            }
        }
    }

    /// `prov`, summarized in a callee, at a call whose arguments have provenance `args`. What the
    /// callee got by invoking a function value it received is what invoking the argument passed
    /// for it gets.
    fn instantiate(&self, prov: &Prov, args: &[Prov]) -> Prov {
        self.instantiate_where(prov, args, &|_| self.strict)
    }

    /// `instantiate`, with `strict` saying per parameter whether an argument nothing is known of
    /// counts as of unknown origin.
    fn instantiate_where(
        &self,
        prov: &Prov,
        args: &[Prov],
        strict: &dyn Fn(usize) -> bool,
    ) -> Prov {
        let mut out = prov.instantiate(args, strict);
        for (p, fun) in prov
            .invoked
            .iter()
            .filter_map(|p| Some((*p, args.get(*p)?)))
        {
            let effect = self.invoke_effect(fun, &out, false, strict(p));
            out.join(&effect);
        }
        out
    }

    /// What a closure target with `summary` returns or stores through its mutable references;
    /// with `sensitive_only`, only the sensitive ones.
    fn effects<'s>(
        &self,
        target: QualifiedId<FunId>,
        summary: &'s Summary,
        sensitive_only: bool,
    ) -> Vec<&'s Prov> {
        let fun = self.env.get_function(target);
        let keep = |ty: &Type| !sensitive_only || sensitive(self.env, ty, self.cap_types, true);
        let results = fun.get_result_type().flatten();
        let mut out: Vec<&Prov> = summary
            .returns
            .iter()
            .flat_map(|(_, provs)| provs.iter())
            .zip(&results)
            .filter(|(_, ty)| keep(ty))
            .map(|(prov, _)| prov)
            .collect();
        if fun
            .get_parameter_types()
            .iter()
            .any(|ty| ty.is_mutable_reference() && keep(ty))
        {
            out.push(&summary.stored);
        }
        out
    }

    /// What invoking a function value with provenance `fun` on operands with joined provenance
    /// `operands` may return or store. An invoke's result is trusted only when its target is
    /// known: each closure target the program made is instantiated with the operands in every
    /// parameter position; a received function value is left to the caller (`invoked`); any
    /// other may return a signer for a system address.
    fn invoke_effect(
        &self,
        fun: &Prov,
        operands: &Prov,
        sensitive_only: bool,
        strict: bool,
    ) -> Prov {
        let mut out = Prov::default();
        // Per target, the operands it was instantiated with; a target runs again only when they
        // grow, so this terminates within the finite lattice.
        let mut seen: BTreeMap<QualifiedId<FunId>, Prov> = BTreeMap::new();
        let mut todo = vec![(fun.clone(), operands.clone())];
        while let Some((fun, operands)) = todo.pop() {
            out.invoked
                .extend(fun.params.iter().copied().filter(|p| self.funs[*p]));
            if self.unknown_fun(&fun, strict) {
                out.origins |= UNKNOWN_CODE;
            }
            for target in &fun.targets {
                if !self.analyzed.contains(target) {
                    out.origins |= UNKNOWN_CODE;
                    continue;
                }
                let operands = match seen.get_mut(target) {
                    Some(prev) => {
                        let mut grown = prev.clone();
                        grown.join(&operands);
                        if grown == *prev {
                            continue;
                        }
                        *prev = grown.clone();
                        grown
                    },
                    None => {
                        seen.insert(*target, operands.clone());
                        operands.clone()
                    },
                };
                let Some(summary) = self.summaries.get(target) else {
                    continue;
                };
                let args = vec![operands; self.env.get_function(*target).get_parameter_count()];
                for prov in self.effects(*target, summary, sensitive_only) {
                    let base = prov.instantiate(&args, &|_| strict);
                    for fun in prov.invoked.iter().filter_map(|q| args.get(*q)) {
                        todo.push((fun.clone(), base.clone()));
                    }
                    out.join(&base);
                }
            }
        }
        out
    }

    /// Whether a function value of provenance `fun` may be one the program did not make and did
    /// not receive in a parameter. Unknown is the default: the only origins a closure the program
    /// made carries are those of what it captured for a known address (hash-derived, a constant
    /// system address, minted for one). Anything else -- storage, a native, what unknown code or
    /// a signer producer returned, a signer minted for an address that may be anyone's -- may
    /// have come with a function value, and so may a value with no target and no parameter to
    /// have come from, which the analysis does not represent.
    fn unknown_fun(&self, fun: &Prov, strict: bool) -> bool {
        fun.origins & !(NON_SYSTEM | SYSTEM_ADDRESS | MINTED_SYSTEM) != 0
            || !fun.produced.is_empty()
            || !fun.invoked.is_empty()
            || (strict && fun.targets.is_empty() && fun.params.iter().all(|p| !self.funs[*p]))
    }

    /// Whether every function value of provenance `fun` is a closure the program made.
    fn known_targets(&self, fun: &Prov) -> bool {
        !self.unknown_fun(fun, self.strict)
            && fun.params.iter().all(|p| !self.funs[*p])
            && fun.targets.iter().all(|t| self.analyzed.contains(t))
    }

    /// Whether handing temporary `t` out hands out authority: it is sensitive, or a mutable
    /// reference into a capability's fields (`CAP_FIELD_REF`).
    fn authority(&self, state: &State, t: TempIndex) -> Option<bool> {
        self.sensitivity(t).or_else(|| {
            (self.types[t].is_mutable_reference() && state.provs[t].origins & CAP_FIELD_REF != 0)
                .then_some(false)
        })
    }

    fn holds_signer(&self, t: TempIndex) -> bool {
        holds_signer(self.env, &self.types[t], self.cap_types)
    }

    /// Whether temporary `t` is sensitive: concretely (`Some(false)`), only through a type
    /// parameter (`Some(true)`), or not (`None`).
    fn sensitivity(&self, t: TempIndex) -> Option<bool> {
        if self.sens[t] {
            Some(false)
        } else if self.gsens[t] {
            Some(true)
        } else {
            None
        }
    }

    /// The joined provenance of the sensitive temporaries among `temps`.
    fn sensitive_join(&self, state: &State, temps: &[TempIndex]) -> Prov {
        let mut out = Prov::default();
        for t in temps {
            if self.sensitivity(*t).is_some() {
                out.join(&state.provs[*t]);
            }
        }
        out
    }

    /// Records what a callee or closure target with `summary`, run on arguments `args`, hands
    /// unknown code, resolved like its returns. `instance` says whether the call's type
    /// arguments are sensitive, concretely (`Some(false)`) or through the caller's own type
    /// parameters (`Some(true)`); `signer_instance`, whether they may hold a signer.
    fn sinks_through(
        &self,
        facts: &mut Facts,
        summary: &Summary,
        args: &[Prov],
        instance: Option<bool>,
        signer_instance: bool,
        strict: &dyn Fn(usize) -> bool,
    ) {
        facts.reach_unknown(
            &self.instantiate_where(&summary.sinks, args, strict),
            false,
            false,
        );
        facts.reach_unknown(
            &self.instantiate_where(&summary.signer_sinks, args, strict),
            false,
            true,
        );
        if let Some(generic) = instance {
            facts.reach_unknown(
                &self.instantiate_where(&summary.generic_sinks, args, strict),
                generic,
                signer_instance,
            );
        }
    }

    /// Applies running a function value with provenance `fun` (`None` for the function a dispatch
    /// native names) on `srcs`, whose provenance joins to `joined`, and returns the provenance of
    /// the results `dests`. A sensitive operand of a function value of unknown target or of a
    /// dispatch native reaches unknown code, which may hand back anything sensitive through a
    /// result or a mutable argument.
    fn run(
        &self,
        state: &mut State,
        mut facts: Option<&mut Facts>,
        fun: Option<&Prov>,
        srcs: &[TempIndex],
        dests: &[TempIndex],
        joined: &Prov,
    ) -> Vec<Prov> {
        let known = fun.is_some_and(|fun| self.known_targets(fun));
        let effect = |sensitive_only: bool| match fun {
            Some(fun) => self.invoke_effect(fun, joined, sensitive_only, self.strict),
            None => Prov::origin(UNKNOWN_CODE),
        };
        let (effect, mut returned) = (effect(false), effect(true));
        // What a received or unknown function value hands back may include its operands.
        if !known {
            returned.join(&self.sensitive_join(state, srcs));
        }
        if let Some(facts) = facts.as_deref_mut() {
            for target in fun.iter().flat_map(|fun| fun.targets.iter()) {
                if let Some(summary) = self.summaries.get(target) {
                    let args =
                        vec![joined.clone(); self.env.get_function(*target).get_parameter_count()];
                    // The target's type arguments are not known here: any operand that may
                    // hold a signer may be what they stand for.
                    let signer = srcs.iter().any(|t| self.holds_signer(*t));
                    self.sinks_through(facts, summary, &args, Some(false), signer, &|_| {
                        self.strict
                    });
                }
            }
            if !known {
                for t in srcs {
                    if let Some(generic) = self.authority(state, *t) {
                        facts.reach_unknown(&state.provs[*t], generic, self.holds_signer(*t));
                    }
                }
            }
        }
        // The function value may store anything it captured or received through a mutable
        // reference it is handed.
        if self.writes_through(srcs) {
            let mut stored = self.sensitive_join(state, srcs);
            stored.join(&returned);
            let mut other = joined.clone();
            other.origins |= OTHER;
            other.join(&effect);
            let cap_addr = other.mint(Value::Other, self.strict);
            self.store(state, facts, &stored, &other, &cap_addr);
        }
        dests
            .iter()
            .map(|t| {
                if self.sensitivity(*t).is_some() {
                    returned.clone()
                } else {
                    let mut prov = joined.clone();
                    prov.origins |= OTHER;
                    prov.join(&effect);
                    prov
                }
            })
            .collect()
    }
}

/// Whether a value of type `ty` may hold a signer, a capability of `caps`, or a function value
/// (which may capture either, and whose invocation may produce a signer from what it captured):
/// through struct fields, type arguments and references; with `type_params`, also as a type
/// parameter.
fn sensitive(env: &GlobalEnv, ty: &Type, caps: &SensitiveTypes, type_params: bool) -> bool {
    match ty {
        Type::TypeParameter(_) => type_params,
        Type::Primitive(PrimitiveType::Signer) | Type::Fun(..) => true,
        Type::Primitive(_) => false,
        Type::Reference(_, ty) | Type::Vector(ty) => sensitive(env, ty, caps, type_params),
        Type::Tuple(tys) => tys.iter().any(|t| sensitive(env, t, caps, type_params)),
        Type::Struct(mid, sid, args) => {
            // A struct holds what its fields hold, with its type arguments substituted, so a
            // phantom argument holds nothing; a struct of a module with natives may also keep
            // values of any argument behind it (a table's values).
            caps.caps.contains(&mid.qualified(*sid))
                || (caps.native_modules.contains(mid)
                    && args.iter().any(|t| sensitive(env, t, caps, type_params)))
                || env
                    .get_struct(mid.qualified(*sid))
                    .get_fields()
                    .any(|f| sensitive(env, &f.get_type().instantiate(args), caps, type_params))
        },
        _ => true,
    }
}

/// Whether a value of type `ty` may hold a capability, through references, vectors, tuples and
/// struct fields with their type arguments substituted, or behind a struct of a module declaring
/// natives (a table's values). Not through a type parameter: only a capability's own module
/// writes its fields, naming its type.
fn holds_cap(env: &GlobalEnv, ty: &Type, caps: &SensitiveTypes) -> bool {
    match ty {
        Type::Reference(_, ty) | Type::Vector(ty) => holds_cap(env, ty, caps),
        Type::Tuple(tys) => tys.iter().any(|t| holds_cap(env, t, caps)),
        Type::Struct(mid, sid, args) => {
            caps.caps.contains(&mid.qualified(*sid))
                || (caps.native_modules.contains(mid)
                    && args.iter().any(|t| holds_cap(env, t, caps)))
                || env
                    .get_struct(mid.qualified(*sid))
                    .get_fields()
                    .any(|f| holds_cap(env, &f.get_type().instantiate(args), caps))
        },
        _ => false,
    }
}

/// What `sensitive` needs of the program: the capabilities a signer can be produced from (what
/// `SIGNER_PRODUCERS` take), and the modules declaring natives.
///
/// A phantom type argument of a plain struct holds no value, which the field substitution in
/// `sensitive` shows; a struct of a module declaring natives counts any type argument, phantom
/// or not, since its natives may keep values of it behind the struct (`Table<phantom K,
/// phantom V>` keeps its values behind its handle, reached through the structural natives).
#[derive(Default)]
struct SensitiveTypes {
    caps: BTreeSet<QualifiedId<StructId>>,
    native_modules: BTreeSet<ModuleId>,
}

/// Whether a value of type `ty` may hold a function value.
fn may_hold_fun(env: &GlobalEnv, ty: &Type) -> bool {
    match ty {
        Type::Primitive(_) => false,
        Type::Reference(_, ty) | Type::Vector(ty) => may_hold_fun(env, ty),
        Type::Tuple(tys) => tys.iter().any(|t| may_hold_fun(env, t)),
        Type::Struct(mid, sid, args) => {
            args.iter().any(|t| may_hold_fun(env, t))
                || env
                    .get_struct(mid.qualified(*sid))
                    .get_fields()
                    .any(|f| may_hold_fun(env, &f.get_type()))
        },
        _ => true,
    }
}

#[allow(clippy::too_many_arguments)]
fn analyze(
    env: &GlobalEnv,
    data: &FunctionData,
    param_count: usize,
    checks: &Checks,
    summaries: &BTreeMap<QualifiedId<FunId>, Summary>,
    analyzed: &BTreeSet<QualifiedId<FunId>>,
    cap_types: &SensitiveTypes,
    strict: bool,
) -> Facts {
    let code = &data.code;
    let types = &data.local_types;
    let ctx = Context {
        env,
        types,
        bears: types.iter().map(|t| may_hold(env, t, true)).collect(),
        checks,
        summaries,
        analyzed,
        cap_types,
        strict,
        sens: types
            .iter()
            .map(|t| sensitive(env, t, cap_types, false))
            .collect(),
        gsens: types
            .iter()
            .map(|t| !sensitive(env, t, cap_types, false) && sensitive(env, t, cap_types, true))
            .collect(),
        funs: types.iter().map(|t| may_hold_fun(env, t)).collect(),
        cap_holders: types.iter().map(|t| holds_cap(env, t, cap_types)).collect(),
        cap_ref_param: (0..param_count)
            .any(|p| types[p].is_mutable_reference() && holds_cap(env, &types[p], cap_types)),
        reachable: {
            let mut reachable: Vec<bool> = types.iter().map(|t| t.is_reference()).collect();
            for bc in code {
                if let Bytecode::Call(_, dests, Operation::BorrowLoc, srcs, _) = bc {
                    if types[dests[0]].is_mutable_reference() {
                        reachable[srcs[0]] = true;
                    }
                }
            }
            reachable
        },
    };
    let entry = State {
        values: (0..types.len())
            .map(|t| {
                if t < param_count
                    && types[t].skip_reference().is_signer()
                    && !types[t].is_mutable_reference()
                {
                    Value::Param(t)
                } else if t < param_count {
                    Value::Other
                } else {
                    Value::Unassigned
                }
            })
            .collect(),
        provs: (0..types.len())
            .map(|t| {
                if t < param_count && ctx.bears[t] {
                    Prov::param(t)
                } else {
                    Prov::default()
                }
            })
            .collect(),
        checked: BTreeSet::new(),
    };
    let cfg = StacklessControlFlowGraph::new_forward(code);
    let first_label = |block: BlockId| {
        cfg.instr_indexes(block)
            .and_then(|mut offsets| offsets.next())
            .and_then(|offset| match &code[offset as usize] {
                Bytecode::Label(_, label) => Some(*label),
                _ => None,
            })
    };
    let mut before: BTreeMap<BlockId, State> = BTreeMap::new();
    before.insert(cfg.entry_block(), entry);
    let mut todo = vec![cfg.entry_block()];
    while let Some(block) = todo.pop() {
        let mut state = before[&block].clone();
        let mut last = None;
        if let Some(offsets) = cfg.instr_indexes(block) {
            for offset in offsets {
                let bc = &code[offset as usize];
                step(&ctx, bc, &mut state, None);
                last = Some(bc);
            }
        }
        for succ in cfg.successors(block) {
            let mut out = state.clone();
            // A branch on whether a parameter's address is a system address checks it on the
            // edge where it is.
            if let Some(Bytecode::Branch(_, then_label, else_label, cond)) = last {
                let label = first_label(*succ);
                match state.values[*cond] {
                    // A branch whose two targets coincide tests nothing.
                    _ if then_label == else_label => {},
                    Value::IsSystem(p) if label == Some(*then_label) => {
                        out.checked.insert(p);
                    },
                    Value::IsNotSystem(p) if label == Some(*else_label) => {
                        out.checked.insert(p);
                    },
                    _ => {},
                }
            }
            let joined = match before.get(succ) {
                Some(old) => old.join(&out),
                None => out,
            };
            if before.get(succ) != Some(&joined) {
                before.insert(*succ, joined);
                todo.push(*succ);
            }
        }
    }
    let mut facts = Facts::default();
    for (block, state) in &before {
        let mut state = state.clone();
        if let Some(offsets) = cfg.instr_indexes(*block) {
            for offset in offsets {
                step(&ctx, &code[offset as usize], &mut state, Some(&mut facts));
            }
        }
    }
    facts.hands_out |= facts.summary.stored.dangerous_at_root()
        && (0..param_count)
            .any(|p| types[p].is_mutable_reference() && ctx.sensitivity(p).is_some());
    facts
}

/// Applies `bc` to `state`, recording what it writes, calls, returns and hands to unknown code
/// in `facts`.
fn step(ctx: &Context, bc: &Bytecode, state: &mut State, mut facts: Option<&mut Facts>) {
    let (env, types, checks) = (ctx.env, ctx.types, ctx.checks);
    let set = |state: &mut State, temps: &[TempIndex], value: Value| {
        for t in temps {
            state.values[*t] = value;
        }
    };
    match bc {
        Bytecode::Assign(_, dest, src, _) => {
            state.values[*dest] = state.values[*src];
            state.provs[*dest] = state.provs[*src].clone();
        },
        Bytecode::Load(_, dest, constant) => {
            state.values[*dest] = match constant {
                Constant::Address(addr) if is_system_address(addr) => Value::SystemAddress,
                _ => Value::Other,
            };
            state.provs[*dest] = Prov::origin(constant_origins(constant));
        },
        Bytecode::Ret(_, temps) => {
            if let Some(facts) = facts {
                let checked = state.checked.clone();
                facts.checked_at_return = Some(match facts.checked_at_return.take() {
                    Some(prev) => prev.intersection(&checked).copied().collect(),
                    None => checked,
                });
                let values: Vec<_> = temps
                    .iter()
                    .map(|t| state.values[*t].summarized())
                    .collect();
                let provs: Vec<_> = temps.iter().map(|t| state.provs[*t].clone()).collect();
                facts.summary.advance(&Summary {
                    returns: Some((values, provs)),
                    ..Summary::default()
                });
                facts.hands_out |= !state.gated()
                    && temps.iter().any(|t| {
                        ctx.authority(state, *t).is_some() && state.provs[*t].dangerous_at_root()
                    });
            }
        },
        Bytecode::Call(_, dests, op, srcs, _) => {
            let args: Vec<Prov> = srcs.iter().map(|t| state.provs[*t].clone()).collect();
            let mut joined = Prov::default();
            for arg in &args {
                joined.join(arg);
            }
            // Set by calls; other operations pass their operands on, and those reading storage
            // or computing a value add `OTHER`.
            let mut dest_provs: Option<Vec<Prov>> = None;
            let value = |t: &TempIndex| state.values[*t];
            match op {
                Operation::BorrowLoc => {
                    let v = value(&srcs[0]);
                    // A local borrowed mutably may be replaced through the reference, and a check
                    // on the signer it held says nothing of its replacement.
                    if types[dests[0]].is_mutable_reference() {
                        if let Value::Param(p) = v {
                            state.checked.remove(&p);
                        }
                        state.values[srcs[0]] = Value::Other;
                        set(state, dests, Value::Other);
                    } else {
                        set(state, dests, v);
                    }
                },
                Operation::ReadRef | Operation::FreezeRef(_) => {
                    let v = value(&srcs[0]);
                    set(state, dests, v)
                },
                Operation::WriteRef => {
                    if ctx.writes_through(&srcs[..1]) {
                        let written = &state.provs[srcs[1]].clone();
                        if ctx.sensitivity(srcs[1]).is_some() {
                            ctx.store(state, facts.as_deref_mut(), written, written, written);
                        } else {
                            // An address written into a capability is judged as a minter's.
                            let cap_addr = if ctx.bears[srcs[1]] {
                                cap_address(written, value(&srcs[1]), ctx.strict)
                            } else {
                                Prov::default()
                            };
                            ctx.store(
                                state,
                                facts.as_deref_mut(),
                                &Prov::default(),
                                written,
                                &cap_addr,
                            );
                        }
                    }
                },
                Operation::Eq | Operation::Neq => {
                    let compared = match (value(&srcs[0]), value(&srcs[1])) {
                        (Value::ParamAddress(p), Value::SystemAddress)
                        | (Value::SystemAddress, Value::ParamAddress(p)) => Some(p),
                        _ => None,
                    };
                    let v = match (compared, op) {
                        (Some(p), Operation::Eq) => Value::IsSystem(p),
                        (Some(p), _) => Value::IsNotSystem(p),
                        (None, _) => Value::Other,
                    };
                    set(state, dests, v)
                },
                Operation::Exists(mid, sid, _) => {
                    let v = match value(&srcs[0]).param() {
                        Some(p) if checks.capabilities.contains(&mid.qualified(*sid)) => {
                            Value::IsSystem(p)
                        },
                        _ => Value::Other,
                    };
                    set(state, dests, v)
                },
                Operation::Not => {
                    let v = match value(&srcs[0]) {
                        Value::IsSystem(p) => Value::IsNotSystem(p),
                        Value::IsNotSystem(p) => Value::IsSystem(p),
                        _ => Value::Other,
                    };
                    set(state, dests, v)
                },
                Operation::MoveTo(mid, sid, _) => {
                    let system_address = match value(&srcs[0]) {
                        Value::Param(p) => state.checked.contains(&p),
                        Value::MintedSystem => true,
                        _ => false,
                    };
                    if let Some(facts) = facts.as_deref_mut() {
                        facts.writes.push(Write {
                            mem: mid.qualified(*sid),
                            publishes: true,
                            system_address,
                            gated: state.gated(),
                        });
                    }
                },
                Operation::MoveFrom(mid, sid, _) | Operation::BorrowGlobal(mid, sid, _) => {
                    let writes = matches!(op, Operation::MoveFrom(..))
                        || types[dests[0]].is_mutable_reference();
                    let system_address = match value(&srcs[0]) {
                        Value::SystemAddress | Value::MintedSystemAddress => true,
                        Value::ParamAddress(p) => state.checked.contains(&p),
                        _ => false,
                    };
                    if writes {
                        if let Some(facts) = facts.as_deref_mut() {
                            facts.writes.push(Write {
                                mem: mid.qualified(*sid),
                                publishes: false,
                                system_address,
                                gated: state.gated(),
                            });
                        }
                    }
                    set(state, dests, Value::Other)
                },
                Operation::Function(mid, fid, type_args) => {
                    let callee = env.get_function(mid.qualified(*fid));
                    let callee_id = callee.get_qualified_id();
                    let name = framework_name(&callee);
                    let name = name.as_deref();
                    if let Some(facts) = facts.as_deref_mut() {
                        facts.calls.push(Call {
                            callee: callee_id,
                            gated: state.gated(),
                            unreceived_signer: srcs.iter().any(|t| {
                                has_signer(&types[*t]) && !matches!(value(t), Value::Param(_))
                            }),
                        });
                        for (_, i) in CONTENT_SINKS.iter().filter(|(n, _)| name == Some(*n)) {
                            if let Some(generic) = srcs.get(*i).and_then(|t| ctx.sensitivity(*t)) {
                                facts.reach_unknown(
                                    &state.provs[srcs[*i]],
                                    generic,
                                    ctx.holds_signer(srcs[*i]),
                                );
                            }
                        }
                    }
                    let arg_values: Vec<Value> = srcs.iter().map(value).collect();
                    let first = arg_values.first().copied().unwrap_or(Value::Other);
                    let is = |names: &[&str]| name.is_some_and(|n| names.contains(&n));
                    let special = match name {
                        Some("signer::address_of" | "signer::borrow_address") => {
                            Some(first.address())
                        },
                        _ if is(MINTERS) => Some(match first {
                            Value::SystemAddress | Value::MintedSystemAddress => {
                                Value::MintedSystem
                            },
                            _ => Value::Other,
                        }),
                        _ if is(IS_SYSTEM) => {
                            Some(first.param().map_or(Value::Other, Value::IsSystem))
                        },
                        _ => None,
                    };
                    let checked_params = checks
                        .functions
                        .get(&callee_id)
                        .into_iter()
                        .flatten()
                        .filter_map(|i| srcs.get(*i).and_then(|t| value(t).param()))
                        .collect::<Vec<_>>();
                    state.checked.extend(checked_params);
                    let summary = ctx.summaries.get(&callee_id);
                    let returns = summary.and_then(|s| s.returns.as_ref());
                    for (i, dest) in dests.iter().enumerate() {
                        state.values[*dest] = match special {
                            Some(v) if i == 0 => v,
                            _ => returns.and_then(|(values, _)| values.get(i)).map_or(
                                if callee.is_native() {
                                    Value::Other
                                } else {
                                    Value::Unassigned
                                },
                                |v| v.instantiate(&arg_values),
                            ),
                        };
                    }
                    let summarized = |i: usize| {
                        returns
                            .and_then(|(_, provs)| provs.get(i))
                            .map(|p| ctx.instantiate(p, &args))
                            .unwrap_or_default()
                    };
                    let mut with_other = joined.clone();
                    with_other.origins |= OTHER;
                    dest_provs = Some(
                        (0..dests.len())
                            .map(|i| {
                                if i == 0 && is(MINTERS) {
                                    args[0].mint(first, ctx.strict)
                                } else if i == 0 && is(SIGNER_PRODUCERS) {
                                    args[0].produce(ctx.strict)
                                } else if is(NON_SYSTEM_PRODUCERS) {
                                    Prov::origin(NON_SYSTEM)
                                } else if is(&["signer::address_of", "signer::borrow_address"]) {
                                    joined.clone()
                                } else if is(JUDGED_ELSEWHERE) {
                                    Prov::origin(CHECKED)
                                } else if is(STRUCTURAL_NATIVES) || is(STRUCTURAL_FUNCTIONS) {
                                    // A sensitive result comes from the container, not a key.
                                    if ctx.sensitivity(dests[i]).is_some() {
                                        ctx.sensitive_join(state, srcs)
                                    } else {
                                        joined.clone()
                                    }
                                } else if callee.is_native() {
                                    with_other.clone()
                                } else {
                                    summarized(i)
                                }
                            })
                            .collect(),
                    );
                    if callee.is_native() {
                        if dispatch_params(env, &callee).is_some() {
                            dest_provs = Some(ctx.run(
                                state,
                                facts.as_deref_mut(),
                                None,
                                srcs,
                                dests,
                                &joined,
                            ));
                        } else if ctx.writes_through(srcs) {
                            let mut moved = ctx.sensitive_join(state, srcs);
                            let mut other = joined.clone();
                            if !is(STRUCTURAL_NATIVES) {
                                other.origins |= OTHER;
                                // A native that only moves its operands is named; any other may
                                // write a sensitive value of its own making where it can.
                                if srcs.iter().any(|t| {
                                    types[*t].is_mutable_reference()
                                        && ctx.sensitivity(*t).is_some()
                                }) {
                                    moved.origins |= UNKNOWN_CODE;
                                }
                            }
                            let cap_addr = other.mint(Value::Other, ctx.strict);
                            ctx.store(state, facts.as_deref_mut(), &moved, &other, &cap_addr);
                        }
                    } else if let Some(summary) = summary {
                        if let Some(facts) = facts.as_deref_mut() {
                            let instance = if type_args
                                .iter()
                                .any(|t| sensitive(env, t, ctx.cap_types, false))
                            {
                                Some(false)
                            } else if type_args
                                .iter()
                                .any(|t| sensitive(env, t, ctx.cap_types, true))
                            {
                                Some(true)
                            } else {
                                None
                            };
                            let signer = type_args
                                .iter()
                                .any(|t| holds_signer(env, t, ctx.cap_types));
                            ctx.sinks_through(facts, summary, &args, instance, signer, &|_| {
                                ctx.strict
                            });
                        }
                        if ctx.writes_through(srcs) {
                            // The summary keeps sensitive stores only; what else the callee writes
                            // is of unknown origin.
                            ctx.store(
                                state,
                                facts.as_deref_mut(),
                                &ctx.instantiate(&summary.stored, &args),
                                &Prov::origin(OTHER),
                                &Prov::origin(MAYBE_SYSTEM),
                            );
                        }
                    }
                },
                Operation::Closure(mid, fid, closure_inst, mask) => {
                    let target = env.get_function(mid.qualified(*fid));
                    let params = target.get_parameter_types();
                    if let Some(facts) = facts.as_deref_mut() {
                        let open = params
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| !mask.is_captured(*i))
                            .map(|(_, ty)| ty.clone())
                            .collect();
                        facts.closures.push((
                            target.get_qualified_id(),
                            open,
                            srcs.iter().any(|t| has_signer(&types[*t])),
                        ));
                    }
                    // A closure holds what it captured, and invoking it may return or store what
                    // its target does with that; unknown code supplies the other arguments.
                    let target_id = target.get_qualified_id();
                    let mut target_args = vec![Prov::default(); params.len()];
                    let captured = (0..params.len()).filter(|i| mask.is_captured(*i));
                    for (i, t) in captured.zip(srcs) {
                        target_args[i] = state.provs[*t].clone();
                    }
                    let mut prov = ctx.sensitive_join(state, srcs);
                    prov.targets.insert(target_id);
                    // Nothing is known of an open parameter here: whoever invokes the closure
                    // supplies it, and is judged for it.
                    let strict = |i: usize| ctx.strict && mask.is_captured(i);
                    if !ctx.analyzed.contains(&target_id) {
                        prov.origins |= UNKNOWN_CODE;
                    } else if let Some(summary) = ctx.summaries.get(&target_id) {
                        if let Some(facts) = facts.as_deref_mut() {
                            let signer = closure_inst
                                .iter()
                                .any(|t| holds_signer(env, t, ctx.cap_types));
                            ctx.sinks_through(
                                facts,
                                summary,
                                &target_args,
                                Some(false),
                                signer,
                                &strict,
                            );
                        }
                        for ret in ctx.effects(target_id, summary, true) {
                            prov.join(&ctx.instantiate_where(ret, &target_args, &strict));
                        }
                    }
                    dest_provs = Some(vec![prov; dests.len()]);
                    set(state, dests, Value::Other)
                },
                Operation::Invoke => {
                    let (fun_value, invoke_args) =
                        srcs.split_last().expect("invoke has a function");
                    let fun = state.provs[*fun_value].clone();
                    dest_provs = Some(ctx.run(
                        state,
                        facts.as_deref_mut(),
                        Some(&fun),
                        invoke_args,
                        dests,
                        &joined,
                    ));
                    if invoke_args.iter().any(|t| has_signer(&types[*t])) {
                        if let (Some(facts), Type::Fun(params, _, _)) =
                            (facts, types[*fun_value].skip_reference())
                        {
                            facts.signer_invokes.push(params.as_ref().clone().flatten());
                        }
                    }
                    set(state, dests, Value::Other)
                },
                // A struct holds what its sensitive fields hold; a capability also its address. A
                // non-sensitive field taken out is of unknown origin, so it still mints as one.
                Operation::Pack(mid, sid, _) | Operation::PackVariant(mid, sid, _, _) => {
                    let mut prov = ctx.sensitive_join(state, srcs);
                    // A field declared with a type parameter may be taken out as one by a
                    // generic function, where nothing marks it as of unknown origin: the struct
                    // keeps what such a field holds.
                    let variant = match op {
                        Operation::PackVariant(_, _, variant, _) => Some(*variant),
                        _ => None,
                    };
                    let open: Vec<bool> = env
                        .get_struct(mid.qualified(*sid))
                        .get_fields_optional_variant(variant)
                        .map(|f| f.get_type().is_open())
                        .collect();
                    if open.len() != srcs.len() {
                        prov.join(&joined);
                    } else {
                        for (t, _) in srcs.iter().zip(&open).filter(|(_, open)| **open) {
                            prov.join(&state.provs[*t]);
                        }
                    }
                    if ctx.cap_types.caps.contains(&mid.qualified(*sid)) {
                        // A capability's address is judged as a minter's: only a hash-derived
                        // one, or a received signer's, is not possibly a system address. Fields
                        // that hold no address (flags) carry nothing.
                        for t in srcs
                            .iter()
                            .filter(|t| ctx.bears[**t] && ctx.sensitivity(**t).is_none())
                        {
                            prov.join(&cap_address(&state.provs[*t], value(t), ctx.strict));
                        }
                    }
                    dest_provs = Some(vec![prov; dests.len()]);
                    set(state, dests, Value::Other)
                },
                Operation::Unpack(..)
                | Operation::UnpackVariant(..)
                | Operation::BorrowField(..)
                | Operation::BorrowVariantField(..)
                | Operation::GetField(..)
                | Operation::GetVariantField(..) => {
                    // A capability keeps all its fields' provenance (`Pack`).
                    let all_kept = match op {
                        Operation::Unpack(mid, sid, _)
                        | Operation::UnpackVariant(mid, sid, _, _)
                        | Operation::BorrowField(mid, sid, _, _)
                        | Operation::BorrowVariantField(mid, sid, _, _, _)
                        | Operation::GetField(mid, sid, _, _)
                        | Operation::GetVariantField(mid, sid, _, _, _) => {
                            ctx.cap_types.caps.contains(&mid.qualified(*sid))
                        },
                        _ => false,
                    };
                    dest_provs = Some(
                        dests
                            .iter()
                            .map(|t| {
                                let mut prov = joined.clone();
                                if !all_kept && ctx.sensitivity(*t).is_none() {
                                    prov.origins |= OTHER;
                                }
                                if all_kept && ctx.types[*t].is_mutable_reference() {
                                    prov.origins |= CAP_FIELD_REF;
                                }
                                prov
                            })
                            .collect(),
                    );
                    set(
                        state,
                        dests,
                        if all_kept {
                            Value::CapAddress
                        } else {
                            Value::Other
                        },
                    )
                },
                _ => set(state, dests, Value::Other),
            }
            let provs = dest_provs.unwrap_or_else(|| {
                let structural = matches!(
                    op,
                    Operation::BorrowLoc
                        | Operation::BorrowField(..)
                        | Operation::BorrowVariantField(..)
                        | Operation::ReadRef
                        | Operation::FreezeRef(_)
                        | Operation::Pack(..)
                        | Operation::PackVariant(..)
                        | Operation::Unpack(..)
                        | Operation::UnpackVariant(..)
                        | Operation::Closure(..)
                        | Operation::Vector
                );
                if !structural {
                    joined.origins |= OTHER;
                }
                vec![joined; dests.len()]
            });
            for (dest, prov) in dests.iter().zip(provs) {
                state.provs[*dest] = if ctx.bears[*dest] {
                    prov
                } else {
                    Prov::default()
                };
            }
        },
        _ => {},
    }
}

/// Whether a function with parameter types `params` may be the target of a function value
/// with parameter types `expected`: equal up to type parameters, which match anything.
fn params_match(params: &[Type], expected: &[Type]) -> bool {
    fn ty_match(a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::TypeParameter(_), _) | (_, Type::TypeParameter(_)) => true,
            (Type::Reference(k1, a), Type::Reference(k2, b)) => k1 == k2 && ty_match(a, b),
            (Type::Vector(a), Type::Vector(b)) => ty_match(a, b),
            (Type::Struct(m1, s1, i1), Type::Struct(m2, s2, i2)) => {
                m1 == m2
                    && s1 == s2
                    && i1.len() == i2.len()
                    && i1.iter().zip(i2).all(|(a, b)| ty_match(a, b))
            },
            (Type::Tuple(a), Type::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| ty_match(a, b))
            },
            (Type::Fun(a1, r1, _), Type::Fun(a2, r2, _)) => ty_match(a1, a2) && ty_match(r1, r2),
            _ => a == b,
        }
    }
    params.len() == expected.len() && params.iter().zip(expected).all(|(a, b)| ty_match(a, b))
}

/// The parameters of the function a dispatch native runs: a native taking a `FunctionInfo`
/// runs the function it names with its other arguments.
fn dispatch_params(env: &GlobalEnv, native: &FunctionEnv) -> Option<Vec<Type>> {
    let is_function_info = |ty: &Type| match ty.skip_reference() {
        Type::Struct(mid, sid, _) => {
            let struct_env = env.get_struct(mid.qualified(*sid));
            matches!(struct_env.module_env.self_address(), Address::Numerical(a) if *a == AccountAddress::ONE)
                && struct_env.get_full_name_str() == "function_info::FunctionInfo"
        },
        _ => false,
    };
    let params = native.get_parameter_types();
    params.iter().any(is_function_info).then(|| {
        params
            .into_iter()
            .filter(|ty| !is_function_info(ty))
            .collect()
    })
}

fn system_address_bound_structs(env: &GlobalEnv) -> BTreeSet<QualifiedId<StructId>> {
    let mut code: Vec<(QualifiedId<FunId>, FunctionData, usize)> = vec![];
    let mut native_modules = BTreeSet::new();
    let mut roots = vec![];
    let mut dispatch_sigs: Vec<Vec<Type>> = vec![];
    let mut base = Checks::default();
    let mut cap_types = SensitiveTypes::default();
    let mut root_natives = vec![];
    for module in env.get_modules() {
        if module.is_script_module() {
            continue;
        }
        let friend_outside = module.get_verified_module().is_some_and(|compiled| {
            compiled
                .immediate_friends()
                .iter()
                .any(|id| env.find_module_by_language_storage_id(id).is_none())
        });
        for fun in module.get_functions() {
            if framework_name(&fun).is_some_and(|n| SIGNER_PRODUCERS.contains(&n.as_str())) {
                if let Some(Type::Struct(mid, sid, _)) = fun
                    .get_parameter_types()
                    .first()
                    .map(|t| t.skip_reference())
                {
                    cap_types.caps.insert(mid.qualified(*sid));
                }
            }
            if framework_name(&fun).is_some_and(|n| CHECKS.contains(&n.as_str())) {
                base.functions
                    .insert(fun.get_qualified_id(), BTreeSet::from([0]));
            }
            if fun.is_native() {
                native_modules.insert(module.get_id());
                cap_types.native_modules.insert(module.get_id());
                if fun.get_parameter_types().iter().any(has_signer) {
                    dispatch_sigs.extend(dispatch_params(env, &fun));
                }
                // Unknown code calls a public native, or a friend one of a module with a friend
                // outside the program, directly.
                match fun.visibility() {
                    Visibility::Public => root_natives.push(fun.get_qualified_id()),
                    Visibility::Friend if friend_outside => {
                        root_natives.push(fun.get_qualified_id())
                    },
                    _ => {},
                }
                continue;
            }
            if fun.is_lemma() || (fun.is_inline() && fun.get_bytecode().is_none()) {
                continue;
            }
            if fun.get_bytecode().is_none() {
                // Code the scan cannot see may write anything.
                return BTreeSet::new();
            }
            match fun.visibility() {
                Visibility::Public => roots.push(fun.get_qualified_id()),
                Visibility::Friend if friend_outside => roots.push(fun.get_qualified_id()),
                _ => {},
            }
            let data = StacklessBytecodeGenerator::new(&fun).generate_function();
            code.push((fun.get_qualified_id(), data, fun.get_parameter_count()));
        }
    }

    // Two least fixpoints, each counting only what the previous round established, so nothing
    // justifies itself. First the summaries, which read no checks: a callee's returned values
    // start at `Unassigned` and only ascend to a value, then `Other` (`Value::join`); its
    // provenance, stores and sinks only grow within finite sets of parameters and origins.
    // Every round is at least the previous one, so the chain is finite. Then checks with the
    // summaries fixed: a checking function or a capability (every `move_to` of it at a system
    // address) found in one round only adds checks in the next, so checks ascend from `base`
    // within the program's finite sets of functions, parameters and structs.
    //
    // The summaries are computed twice over. Until every callee's summary is complete, a value
    // with no provenance may be one a later round resolves, so the first pass takes it for
    // nothing. In the second pass, which starts from the first one's fixpoint, such a value is
    // one the analysis does not represent, and what consumes it as authority takes it for a value
    // of unknown origin (`Prov::unrepresented`); this only adds, so the chain stays ascending.
    let analyzed: BTreeSet<_> = code.iter().map(|(qid, _, _)| *qid).collect();
    let analyze_all =
        |checks: &Checks, summaries: &BTreeMap<QualifiedId<FunId>, Summary>, strict: bool| {
            code.iter()
                .map(|(qid, data, params)| {
                    let facts = analyze(
                        env, data, *params, checks, summaries, &analyzed, &cap_types, strict,
                    );
                    (*qid, facts)
                })
                .collect::<BTreeMap<QualifiedId<FunId>, Facts>>()
        };
    let mut summaries: BTreeMap<QualifiedId<FunId>, Summary> = BTreeMap::new();
    for strict in [false, true] {
        loop {
            let mut next = summaries.clone();
            for (qid, f) in analyze_all(&base, &summaries, strict) {
                next.entry(qid).or_default().advance(&f.summary);
            }
            if next == summaries {
                break;
            }
            summaries = next;
        }
    }
    let mut checks = base.clone();
    let facts = loop {
        let facts = analyze_all(&checks, &summaries, true);
        let mut next = base.clone();
        for (qid, f) in &facts {
            if let Some(checked) = f.checked_at_return.as_ref().filter(|c| !c.is_empty()) {
                next.functions
                    .entry(*qid)
                    .or_default()
                    .extend(checked.iter().copied());
            }
        }
        let published_elsewhere: BTreeSet<_> = facts
            .values()
            .flat_map(|f| f.writes.iter())
            .filter(|w| w.publishes && !w.system_address)
            .map(|w| w.mem)
            .collect();
        next.capabilities = env
            .get_modules()
            .filter(|m| !m.is_script_module() && !native_modules.contains(&m.get_id()))
            .flat_map(|m| {
                m.get_structs()
                    .map(|s| s.get_qualified_id())
                    .collect::<Vec<_>>()
            })
            .filter(|s| !published_elsewhere.contains(s))
            .collect();
        if next == checks {
            break facts;
        }
        checks = next;
    };

    let mut minted_roots = vec![];
    for f in facts.values() {
        dispatch_sigs.extend(f.signer_invokes.iter().cloned());
        for (target, _, captures_signer) in &f.closures {
            roots.push(*target);
            if *captures_signer {
                minted_roots.push(*target);
            }
        }
    }
    let hands_out_by_design = |qid: &QualifiedId<FunId>| {
        framework_name(&env.get_function(*qid)).is_some_and(|n| {
            [
                TRUSTED_SIGNER_SOURCES,
                SIGNER_PRODUCERS,
                STRUCTURAL_FUNCTIONS,
                JUDGED_ELSEWHERE,
            ]
            .iter()
            .any(|names| names.contains(&n.as_str()))
        })
    };
    // Unknown code instantiates a root's type parameters, so a value sensitive only through one
    // counts as sensitive there.
    let root_leaks = |root: &QualifiedId<FunId>| {
        facts.get(root).is_some_and(|f| {
            (f.hands_out && !hands_out_by_design(root))
                || f.summary.generic_sinks.dangerous_at_root()
                || !f.summary.sinks.minted_from.is_empty()
        })
    };
    // What a root native returns, or stores through a mutable parameter, is of unknown origin;
    // a sensitive such value hands out, except from the natives that only move data, the named
    // producers, and the natives judged elsewhere. A type parameter counts: unknown code
    // instantiates it, and a native not in those lists may build a value of it from nothing.
    let native_hands_out = |qid: &QualifiedId<FunId>| {
        let fun = env.get_function(*qid);
        let exempt = framework_name(&fun).is_some_and(|n| {
            [
                STRUCTURAL_NATIVES,
                NON_SYSTEM_PRODUCERS,
                TRUSTED_SIGNER_SOURCES,
                JUDGED_ELSEWHERE,
            ]
            .iter()
            .any(|names| names.contains(&n.as_str()))
        });
        let sens = |ty: &Type| sensitive(env, ty, &cap_types, true);
        !exempt
            && (fun.get_result_type().flatten().iter().any(sens)
                || fun
                    .get_parameter_types()
                    .iter()
                    .any(|ty| ty.is_mutable_reference() && sens(ty)))
    };
    // A signer a function received may be one for a system address where its caller may be the
    // VM or a governance script, which hold such signers: a function unknown code can enter
    // (a script calls what unknown code calls), an `entry` function, and a function nothing in
    // the program calls. Elsewhere the callers resolve it from what they pass. Such a signer
    // must not reach unknown code, which could pass any check with it; only one for a
    // hash-derived address may.
    let called: BTreeSet<QualifiedId<FunId>> = facts
        .values()
        .flat_map(|f| f.calls.iter().map(|c| c.callee))
        .collect();
    let outside_callable = |qid: &QualifiedId<FunId>| {
        roots.contains(qid) || !called.contains(qid) || env.get_function(*qid).is_entry()
    };
    let hands_signer_on = |(qid, f): (&QualifiedId<FunId>, &Facts)| {
        let sinks = &f.summary.signer_sinks;
        outside_callable(qid)
            && (!sinks.params.is_empty()
                || !sinks.produced.is_empty()
                || !sinks.invoked.is_empty()
                || !sinks.minted_from.is_empty())
    };
    if facts.values().any(|f| f.leaks)
        || facts.iter().any(hands_signer_on)
        || roots.iter().any(root_leaks)
        || root_natives.iter().any(native_hands_out)
    {
        return BTreeSet::new();
    }
    for sig in &dispatch_sigs {
        for qid in facts.keys() {
            if params_match(&env.get_function(*qid).get_parameter_types(), sig) {
                minted_roots.push(*qid);
            }
        }
        for f in facts.values() {
            for (target, open, _) in &f.closures {
                if params_match(open, sig) {
                    minted_roots.push(*target);
                }
            }
        }
    }

    // `U` (reached past no check) and `U*` (reached with a minted signer: checks count for
    // nothing).
    let mut reached = BTreeSet::new();
    let mut reached_minted = BTreeSet::new();
    let mut todo: Vec<(QualifiedId<FunId>, bool)> = roots
        .into_iter()
        .map(|f| (f, false))
        .chain(minted_roots.into_iter().map(|f| (f, true)))
        .collect();
    while let Some((f, minted)) = todo.pop() {
        if minted {
            if !reached_minted.insert(f) {
                continue;
            }
            reached.insert(f);
        } else if reached_minted.contains(&f) || !reached.insert(f) {
            continue;
        }
        for call in facts
            .get(&f)
            .map(|f| f.calls.as_slice())
            .unwrap_or_default()
        {
            if minted {
                todo.push((call.callee, true));
            } else if !call.gated {
                todo.push((call.callee, call.unreceived_signer));
            }
        }
    }

    let mut disqualified = BTreeSet::new();
    for (f, fun_facts) in &facts {
        for write in &fun_facts.writes {
            let protected = !reached.contains(f) || (!reached_minted.contains(f) && write.gated);
            if !write.system_address || !protected {
                disqualified.insert(write.mem);
            }
        }
    }
    env.get_modules()
        .filter(|m| !m.is_script_module() && !native_modules.contains(&m.get_id()))
        .filter(|m| m.get_verified_module().is_some())
        .flat_map(|m| {
            m.get_structs()
                .map(|s| s.get_qualified_id())
                .collect::<Vec<_>>()
        })
        .filter(|s| !disqualified.contains(s))
        .collect()
}

/// The provenance a capability gets from an address with provenance `prov` and value `value`: as
/// a minter's input, so only a hash-derived address, or a received signer's or capability's, is
/// not possibly a system address.
fn cap_address(prov: &Prov, value: Value, strict: bool) -> Prov {
    match value {
        Value::CapAddress => prov.clone(),
        Value::SystemAddress | Value::MintedSystemAddress | Value::ParamAddress(_) => {
            prov.mint(value, strict)
        },
        _ => prov.minted(strict),
    }
}

/// Where the addresses a constant holds come from: a vector of addresses holds each of them. An
/// address that is not numerical is not known not to be a system address.
fn constant_origins(constant: &Constant) -> u8 {
    let of = |addr: &Address| match addr {
        _ if is_system_address(addr) => SYSTEM_ADDRESS,
        Address::Numerical(_) => NON_SYSTEM,
        _ => OTHER,
    };
    match constant {
        Constant::Address(addr) => of(addr),
        Constant::AddressArray(addrs) => addrs.iter().fold(0, |origins, a| origins | of(a)),
        Constant::Vector(elems) => elems
            .iter()
            .fold(0, |origins, c| origins | constant_origins(c)),
        _ => 0,
    }
}
