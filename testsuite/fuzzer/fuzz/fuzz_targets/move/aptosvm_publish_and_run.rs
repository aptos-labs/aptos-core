#![no_main]

// Copyright © Aptos Foundation

use aptos_language_e2e_tests::{
    account::Account, data_store::GENESIS_CHANGE_SET_HEAD, executor::FakeExecutor,
};
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        EntryFunction, ExecutionStatus, ModuleBundle, Script, TransactionArgument,
        TransactionPayload, TransactionStatus,
    },
    write_set::WriteSet,
};
use aptos_vm::AptosVM;
use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledModule, CompiledScript, FunctionDefinitionIndex},
};
use move_core_types::{
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
    vm_status::{StatusCode, StatusType, VMStatus},
};
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashSet},
    convert::TryInto,
};

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone, Copy)]
pub enum FundAmount {
    Zero,
    Poor,
    Rich,
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone, Copy)]
pub struct UserAccount {
    is_inited_and_funded: bool,
    fund: FundAmount,
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub enum Authenticator {
    Ed25519 {
        sender: UserAccount,
    },
    MultiAgent {
        sender: UserAccount,
        secondary_signers: Vec<UserAccount>,
    },
    FeePayer {
        sender: UserAccount,
        secondary_signers: Vec<UserAccount>,
        fee_payer: UserAccount,
    },
}

impl UserAccount {
    fn fund_amount(&self) -> u64 {
        match self.fund {
            FundAmount::Zero => 0,
            FundAmount::Poor => 1_000,
            FundAmount::Rich => 1_000_000_000_000_000,
        }
    }

    fn convert_account(&self, vm: &mut FakeExecutor) -> Account {
        if self.is_inited_and_funded {
            vm.create_accounts(1, self.fund_amount(), 0).remove(0)
        } else {
            Account::new()
        }
    }
}

impl Authenticator {
    fn sender(&self) -> UserAccount {
        match self {
            Authenticator::Ed25519 { sender } => *sender,
            Authenticator::MultiAgent {
                sender,
                secondary_signers: _,
            } => *sender,
            Authenticator::FeePayer {
                sender,
                secondary_signers: _,
                fee_payer: _,
            } => *sender,
        }
    }
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub enum ExecVariant {
    Script {
        script: CompiledScript,
        type_args: Vec<TypeTag>,
        args: Vec<MoveValue>,
    },
    CallFunction {
        module: ModuleId,
        function: FunctionDefinitionIndex,
        type_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    },
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub struct RunnableState {
    pub dep_modules: Vec<CompiledModule>,
    pub exec_variant: ExecVariant,
    pub tx_auth_type: Authenticator,
}

// genesis write set generated once for each fuzzing session
static VM: Lazy<WriteSet> = Lazy::new(|| GENESIS_CHANGE_SET_HEAD.write_set().clone());

// small debug macro which can be enabled or disabled
const DEBUG: bool = false;
macro_rules! tdbg {
    () => {
        ()
    };
    ($val:expr $(,)?) => {
        if DEBUG {
            dbg!($val)
        } else {
            ($val)
        }
    };
    ($($val:expr),+ $(,)?) => {
        if DEBUG {
            dbg!($(($val)),+,)
        } else {
            ($(($val)),+,)
        }
    };
}

// used for ordering modules topologically
fn sort_by_deps(
    map: &BTreeMap<ModuleId, CompiledModule>,
    order: &mut Vec<ModuleId>,
    id: ModuleId,
    visited: &mut HashSet<ModuleId>,
) -> Result<(), Corpus> {
    if visited.contains(&id) {
        return Err(Corpus::Keep);
    }
    visited.insert(id.clone());
    if order.contains(&id) {
        return Ok(());
    }
    let compiled = &map.get(&id).unwrap();
    for dep in compiled.immediate_dependencies() {
        // Only consider deps which are actually in this package. Deps for outside
        // packages are considered fine because of package deployment order. Note
        // that because of this detail, we can't use existing topsort from Move utils.
        if map.contains_key(&dep) {
            sort_by_deps(map, order, dep, visited)?;
        }
    }
    order.push(id);
    Ok(())
}

// panic to catch invariant violations
fn check_for_invariant_violation(e: VMStatus) {
    if e.status_type() == StatusType::InvariantViolation {
        // known false positive
        if e.message() != Some(&"moving container with dangling references".to_string()) {
            panic!("invariant violation {:?}", e);
        }
    }
}

fn run_case(mut input: RunnableState) -> Result<(), Corpus> {
    tdbg!(&input);
    AptosVM::set_concurrency_level_once(2);
    let mut vm = FakeExecutor::from_genesis(&VM, ChainId::mainnet()).set_not_parallel();

    for m in input.dep_modules.iter_mut() {
        // m.metadata = vec![]; // we could optimize metadata to only contain aptos metadata
        m.version = 6; // others don't matter
    }
    for module in input.dep_modules.iter() {
        // reject bad modules fast
        move_bytecode_verifier::verify_module(module).map_err(|_| Corpus::Keep)?;
    }

    // check no duplicates
    let mset: HashSet<_> = input.dep_modules.iter().map(|m| m.self_id()).collect();
    if mset.len() != input.dep_modules.len() {
        return Err(Corpus::Keep);
    }

    // topologically order modules {
    let all_modules = input.dep_modules.clone();
    let mut map = all_modules
        .into_iter()
        .map(|m| (m.self_id(), m))
        .collect::<BTreeMap<_, _>>();
    let mut order = vec![];
    for id in map.keys() {
        let mut visited = HashSet::new();
        sort_by_deps(&map, &mut order, id.clone(), &mut visited)?;
    }
    // }

    // group same address modules in packages. keep local ordering.
    let mut packages = vec![];
    for cur_package_id in order.iter() {
        let mut cur = vec![];
        if !map.contains_key(cur_package_id) {
            continue;
        }
        // this makes sure we keep the order in packages
        for id in order.iter() {
            // check if part of current package
            if id.address() == cur_package_id.address() {
                if let Some(module) = map.remove(cur_package_id) {
                    cur.push(module);
                }
            }
        }
        packages.push(cur)
    }

    // publish all packages
    for group in packages {
        let sender = *group[0].address();
        let serialized_modules: Vec<Vec<u8>> = group
            .iter()
            .map(|m| {
                let mut b = vec![];
                m.serialize(&mut b).map(|_| b)
            })
            .collect::<Result<Vec<Vec<u8>>, _>>()
            .map_err(|_| Corpus::Keep)?;

        // deprecated but easiest way to publish modules
        let mb = ModuleBundle::new(serialized_modules);
        let acc = vm.new_account_at(sender);
        let tx = acc
            .transaction()
            .gas_unit_price(100)
            .sequence_number(0)
            .payload(TransactionPayload::ModuleBundle(mb))
            .sign();
        tdbg!("publishing");
        let res = vm
            .execute_block(vec![tx])
            .map_err(|e| {
                check_for_invariant_violation(e);
                Corpus::Keep
            })?
            .pop()
            .expect("expected 1 output");
        tdbg!(&res);
        // if error exit gracefully
        let status = match tdbg!(res.status()) {
            TransactionStatus::Keep(status) => status,
            _ => return Err(Corpus::Keep),
        };
        vm.apply_write_set(res.write_set());
        match tdbg!(status) {
            ExecutionStatus::Success => (),
            ExecutionStatus::MiscellaneousError(e) => {
                if let Some(e) = e {
                    if e.status_type() == StatusType::InvariantViolation {
                        panic!("invariant violation {:?}", e);
                    }
                }
                return Err(Corpus::Keep);
            },
            _ => return Err(Corpus::Keep),
        };
        tdbg!("published");
    }

    let sender_acc = if true {
        // create sender pub/priv key. initialize and fund account
        vm.create_accounts(1, input.tx_auth_type.sender().fund_amount(), 0)
            .remove(0)
    } else {
        // only create sender pub/priv key. do not initialize
        Account::new()
    };
    // build tx
    let tx = match input.exec_variant.clone() {
        ExecVariant::Script {
            script,
            type_args,
            args,
        } => {
            let mut script_bytes = vec![];
            script
                .serialize(&mut script_bytes)
                .map_err(|_| Corpus::Keep)?;
            sender_acc
                .transaction()
                .gas_unit_price(100)
                .max_gas_amount(1000)
                .sequence_number(0)
                .payload(TransactionPayload::Script(Script::new(
                    script_bytes,
                    type_args,
                    args.into_iter()
                        .map(|x| x.try_into())
                        .collect::<Result<Vec<TransactionArgument>, _>>()
                        .map_err(|_| Corpus::Keep)?,
                )))
        },
        ExecVariant::CallFunction {
            module,
            function,
            type_args,
            args,
        } => {
            // convert FunctionDefinitionIndex to function name... {
            let cm = input
                .dep_modules
                .iter()
                .find(|m| m.self_id() == module)
                .ok_or(Corpus::Keep)?;
            let fhi = cm
                .function_defs
                .get(function.0 as usize)
                .ok_or(Corpus::Keep)?
                .function;
            let function_identifier_index = cm
                .function_handles
                .get(fhi.0 as usize)
                .ok_or(Corpus::Keep)?
                .name;
            let function_name = cm
                .identifiers
                .get(function_identifier_index.0 as usize)
                .ok_or(Corpus::Keep)?
                .clone();
            // }
            sender_acc
                .transaction()
                .gas_unit_price(100)
                .max_gas_amount(1000)
                .sequence_number(0)
                .payload(TransactionPayload::EntryFunction(EntryFunction::new(
                    module,
                    function_name,
                    type_args,
                    args,
                )))
        },
    };

    let raw_tx = tx.raw();
    let tx = match input.tx_auth_type {
        Authenticator::Ed25519 { sender: _ } => raw_tx
            .sign(&sender_acc.privkey, sender_acc.pubkey)
            .map_err(|_| Corpus::Keep)?
            .into_inner(),
        Authenticator::MultiAgent {
            sender: _,
            secondary_signers,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Keep);
            }
            let secondary_accs: Vec<_> = secondary_signers
                .iter()
                .map(|acc| acc.convert_account(&mut vm))
                .collect();
            let secondary_signers = secondary_accs.iter().map(|acc| *acc.address()).collect();
            let secondary_private_keys = secondary_accs.iter().map(|acc| &acc.privkey).collect();
            raw_tx
                .sign_multi_agent(
                    &sender_acc.privkey,
                    secondary_signers,
                    secondary_private_keys,
                )
                .map_err(|_| Corpus::Keep)?
                .into_inner()
        },
        Authenticator::FeePayer {
            sender: _,
            secondary_signers,
            fee_payer,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Keep);
            }
            let secondary_accs: Vec<_> = secondary_signers
                .iter()
                .map(|acc| acc.convert_account(&mut vm))
                .collect();

            let secondary_signers = secondary_accs.iter().map(|acc| *acc.address()).collect();
            let secondary_private_keys = secondary_accs.iter().map(|acc| &acc.privkey).collect();
            let fee_payer_acc = fee_payer.convert_account(&mut vm);
            raw_tx
                .sign_fee_payer(
                    &sender_acc.privkey,
                    secondary_signers,
                    secondary_private_keys,
                    *fee_payer_acc.address(),
                    &fee_payer_acc.privkey,
                )
                .map_err(|_| Corpus::Keep)?
                .into_inner()
        },
    };

    // exec tx
    tdbg!("exec start");
    let mut old_res = None;
    const N_EXTRA_RERUNS: usize = 3;
    for _ in 0..N_EXTRA_RERUNS {
        let res = vm.execute_block(vec![tx.clone()]);
        if let Some(old_res) = old_res {
            assert!(old_res == res);
        }
        old_res = Some(res);
    }
    let res = vm.execute_block(vec![tx]);
    // check main execution as well
    if let Some(old_res) = old_res {
        assert!(old_res == res);
    }
    let res = res
        .map_err(|e| {
            check_for_invariant_violation(e);
            Corpus::Keep
        })?
        .pop()
        .expect("expect 1 output");
    tdbg!("exec end");

    // if error exit gracefully
    let status = match tdbg!(res.status()) {
        TransactionStatus::Keep(status) => status,
        TransactionStatus::Discard(e) => {
            if e.status_type() == StatusType::InvariantViolation {
                panic!("invariant violation {:?}", e);
            }
            return Err(Corpus::Keep);
        },
        _ => return Err(Corpus::Keep),
    };
    match tdbg!(status) {
        ExecutionStatus::Success => (),
        ExecutionStatus::MiscellaneousError(e) => {
            if let Some(e) = e {
                if e.status_type() == StatusType::InvariantViolation
                    && *e != StatusCode::TYPE_RESOLUTION_FAILURE
                    && *e != StatusCode::STORAGE_ERROR
                {
                    panic!("invariant violation {:?}", e);
                }
            }
            return Err(Corpus::Keep);
        },
        _ => return Err(Corpus::Keep),
    };

    Ok(())
}

fuzz_target!(|fuzz_data: RunnableState| -> Corpus {
    run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
});
