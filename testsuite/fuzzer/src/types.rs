// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use arbitrary::Arbitrary;
use dearbitrary::Dearbitrary;
use move_binary_format::file_format::{CompiledModule, CompiledScript, FunctionDefinitionIndex};
use move_core_types::{
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
};
use move_transactional_test_runner::transactional_ops::TransactionalOperation;

// Originally from testsuite/fuzzer/fuzz/fuzz_targets/move/utils/helpers.rs
// (with Dearbitrary added and fields made public)
#[derive(Debug, Arbitrary, Dearbitrary, Eq, PartialEq, Clone, Copy)]
pub enum FundAmount {
    Zero,
    Poor,
    Rich,
}

// Originally from testsuite/fuzzer/fuzz/fuzz_targets/move/utils/helpers.rs
// (with Dearbitrary added and fields made public)
#[derive(Debug, Arbitrary, Dearbitrary, Eq, PartialEq, Clone, Copy)]
pub struct UserAccount {
    pub is_inited_and_funded: bool,
    pub fund: FundAmount,
}

impl UserAccount {
    pub fn fund_amount(&self) -> u64 {
        match self.fund {
            FundAmount::Zero => 0,
            FundAmount::Poor => 1_000,
            FundAmount::Rich => 1_000_000_000_000_000,
        }
    }

    pub fn convert_account(&self, vm: &mut FakeExecutor) -> Account {
        if self.is_inited_and_funded {
            vm.create_accounts(1, self.fund_amount(), Some(0)).remove(0)
        } else {
            Account::new()
        }
    }
}

// Originally from testsuite/fuzzer/src/utils.rs
#[derive(Debug, Eq, PartialEq, Clone, Arbitrary, Dearbitrary)]
pub enum ExecVariant {
    Script {
        _script: CompiledScript,
        _type_args: Vec<TypeTag>,
        _args: Vec<MoveValue>,
    },
    CallFunction {
        _module: ModuleId,
        _function: FunctionDefinitionIndex,
        _type_args: Vec<TypeTag>,
        _args: Vec<Vec<u8>>,
    },
}

// Originally from testsuite/fuzzer/src/utils.rs
#[derive(Debug, Arbitrary, Dearbitrary, Eq, PartialEq, Clone)]
pub enum Authenticator {
    Ed25519 {
        _sender: UserAccount,
    },
    MultiAgent {
        _sender: UserAccount,
        _secondary_signers: Vec<UserAccount>,
    },
    FeePayer {
        _sender: UserAccount,
        _secondary_signers: Vec<UserAccount>,
        _fee_payer: UserAccount,
    },
}

impl Authenticator {
    pub fn sender(&self) -> UserAccount {
        match self {
            Authenticator::Ed25519 { _sender } => *_sender,
            Authenticator::MultiAgent { _sender, .. } => *_sender,
            Authenticator::FeePayer { _sender, .. } => *_sender,
        }
    }
}

// Originally from testsuite/fuzzer/src/utils.rs
#[derive(Debug, Eq, PartialEq, Clone, Arbitrary, Dearbitrary)]
pub struct RunnableState {
    pub dep_modules: Vec<CompiledModule>,
    pub exec_variant: ExecVariant,
    pub tx_auth_type: Authenticator,
}

// Originally from testsuite/fuzzer/src/utils.rs
#[derive(Debug, Eq, PartialEq, Clone, Arbitrary, Dearbitrary)]
pub struct RunnableStateWithOperations {
    pub operations: Vec<TransactionalOperation>,
    pub tx_auth_type: Authenticator,
}
