// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::format_err;
use aptos_crypto::HashValue;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{self, aptos_test_root_address},
    on_chain_config::{Features, TimedFeaturesBuilder},
    state_store::StateView,
    transaction::{ChangeSet, Script, Version},
};
use aptos_vm::{
    data_cache::AsMoveResolver,
    move_vm_ext::{MoveVmExt, SessionExt, SessionId},
};
use aptos_vm_types::storage::change_set_configs::ChangeSetConfigs;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    transaction_argument::convert_txn_args,
    value::{serialize_values, MoveValue},
};
use move_vm_runtime::session::SerializedReturnValues;
use move_vm_types::gas::UnmeteredGasMeter;

pub struct GenesisSession<'r, 'l>(SessionExt<'r, 'l>);

impl<'r, 'l> GenesisSession<'r, 'l> {
    pub fn exec_func(
        &mut self,
        module_name: &str,
        function_name: &str,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
    ) {
        self.0
            .execute_function_bypass_visibility(
                &ModuleId::new(
                    account_config::CORE_CODE_ADDRESS,
                    Identifier::new(module_name).unwrap(),
                ),
                &Identifier::new(function_name).unwrap(),
                ty_args,
                args,
                &mut UnmeteredGasMeter,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "Error calling {}.{}: {}",
                    module_name,
                    function_name,
                    e.into_vm_status()
                )
            });
    }

    pub fn exec_script(
        &mut self,
        sender: AccountAddress,
        script: &Script,
    ) -> SerializedReturnValues {
        let mut temp = vec![sender.to_vec()];
        temp.extend(convert_txn_args(script.args()));
        self.0
            .execute_script(
                script.code().to_vec(),
                script.ty_args().to_vec(),
                temp,
                &mut UnmeteredGasMeter,
            )
            .unwrap()
    }

    fn disable_reconfiguration(&mut self) {
        self.exec_func(
            "Reconfiguration",
            "disable_reconfiguration",
            vec![],
            serialize_values(&vec![MoveValue::Signer(aptos_test_root_address())]),
        )
    }

    fn enable_reconfiguration(&mut self) {
        self.exec_func(
            "Reconfiguration",
            "enable_reconfiguration",
            vec![],
            serialize_values(&vec![MoveValue::Signer(aptos_test_root_address())]),
        )
    }

    pub fn set_aptos_version(&mut self, version: Version) {
        self.exec_func(
            "AptosVersion",
            "set_version",
            vec![],
            serialize_values(&vec![
                MoveValue::Signer(aptos_test_root_address()),
                MoveValue::U64(version),
            ]),
        )
    }
}

pub fn build_changeset<S: StateView, F>(state_view: &S, procedure: F, chain_id: u8) -> ChangeSet
where
    F: FnOnce(&mut GenesisSession),
{
    let resolver = state_view.as_move_resolver();
    let move_vm = MoveVmExt::new(
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        LATEST_GAS_FEATURE_VERSION,
        chain_id,
        Features::default(),
        TimedFeaturesBuilder::enable_all().build(),
        &resolver,
    )
    .unwrap();
    let change_set = {
        // TODO: specify an id by human and pass that in.
        let genesis_id = HashValue::zero();
        let mut session =
            GenesisSession(move_vm.new_session(&resolver, SessionId::genesis(genesis_id)));
        session.disable_reconfiguration();
        procedure(&mut session);
        session.enable_reconfiguration();
        session
            .0
            .finish(&ChangeSetConfigs::unlimited_at_gas_feature_version(
                LATEST_GAS_FEATURE_VERSION,
            ))
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))
            .unwrap()
    };

    // Genesis never produces the delta change set.
    assert!(change_set.aggregator_v1_delta_set().is_empty());
    change_set
        .try_into_storage_change_set()
        .expect("Conversion from VMChangeSet into ChangeSet should always succeed")
}
