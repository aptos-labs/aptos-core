// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::format_err;
use aptos_crypto::HashValue;
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use aptos_state_view::StateView;
use aptos_types::on_chain_config::{FeatureFlag, Features};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{self, aptos_test_root_address},
    transaction::{ChangeSet, Script, Version},
};
use aptos_vm::{
    data_cache::StorageAdapter,
    move_vm_ext::{MoveResolverExt, MoveVmExt, SessionExt, SessionId},
};
use move_deps::{
    move_core_types::{
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
        transaction_argument::convert_txn_args,
        value::{serialize_values, MoveValue},
    },
    move_vm_runtime::session::SerializedReturnValues,
    move_vm_types::gas::UnmeteredGasMeter,
};

pub struct GenesisSession<'r, 'l, S>(SessionExt<'r, 'l, S>);

impl<'r, 'l, S: MoveResolverExt> GenesisSession<'r, 'l, S> {
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

pub fn build_changeset<S: StateView, F>(state_view: &S, procedure: F) -> ChangeSet
where
    F: FnOnce(&mut GenesisSession<StorageAdapter<S>>),
{
    let move_vm = MoveVmExt::new(
        NativeGasParameters::zeros(),
        AbstractValueSizeGasParameters::zeros(),
        Features::default().is_enabled(FeatureFlag::TREAT_FRIEND_AS_PRIVATE),
    )
    .unwrap();
    let state_view_storage = StorageAdapter::new(state_view);
    let session_out = {
        // TODO: specify an id by human and pass that in.
        let genesis_id = HashValue::zero();
        let mut session = GenesisSession(
            move_vm.new_session(&state_view_storage, SessionId::genesis(genesis_id)),
        );
        session.disable_reconfiguration();
        procedure(&mut session);
        session.enable_reconfiguration();
        session
            .0
            .finish()
            .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))
            .unwrap()
    };

    // Genesis never produces the delta change set.
    let (_, change_set) = session_out
        .into_change_set(&mut ())
        .map_err(|err| format_err!("Unexpected VM Error: {:?}", err))
        .unwrap()
        .into_inner();
    change_set
}
