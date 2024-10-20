// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::BlockExecutableTransaction as Transaction;
use std::{collections::HashSet, fmt};

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum InputOutputKey<K, T, I> {
    Resource(K),
    Group(K, T),
    DelayedField(I),
}

pub struct ReadWriteSummary<T: Transaction> {
    reads: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
    writes: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
}

impl<T: Transaction> ReadWriteSummary<T> {
    pub fn new(
        reads: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
        writes: HashSet<InputOutputKey<T::Key, T::Tag, T::Identifier>>,
    ) -> Self {
        Self { reads, writes }
    }

    pub fn conflicts_with_previous(&self, previous: &Self) -> bool {
        !self.reads.is_disjoint(&previous.writes)
    }

    pub fn collapse_resource_group_conflicts(self) -> Self {
        let collapse = |k: InputOutputKey<T::Key, T::Tag, T::Identifier>| match k {
            InputOutputKey::Resource(k) => InputOutputKey::Resource(k),
            InputOutputKey::Group(k, _) => InputOutputKey::Resource(k),
            InputOutputKey::DelayedField(id) => InputOutputKey::DelayedField(id),
        };
        Self {
            reads: self.reads.into_iter().map(collapse).collect(),
            writes: self.writes.into_iter().map(collapse).collect(),
        }
    }
}

impl<T: Transaction> fmt::Debug for ReadWriteSummary<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ReadWriteSummary")?;
        writeln!(f, "reads:")?;
        for read in &self.reads {
            writeln!(f, "    {:?}", read)?;
        }
        writeln!(f, "writes:")?;
        for write in &self.writes {
            writeln!(f, "    {:?}", write)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test_types {
    use aptos_mvhashmap::types::TxnIndex;
    use aptos_types::{state_store::state_value::StateValue, vm::modules::AptosModuleExtension};
    use bytes::Bytes;
    use move_binary_format::{
        file_format::empty_module_with_dependencies_and_friends, CompiledModule,
    };
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };
    use move_vm_runtime::{Module, RuntimeEnvironment};
    use move_vm_types::code::{MockDeserializedCode, MockVerifiedCode, ModuleCode};
    use std::sync::Arc;

    pub(crate) fn mock_deserialized_code(
        value: usize,
        version: Option<TxnIndex>,
    ) -> Arc<ModuleCode<MockDeserializedCode, MockVerifiedCode, (), Option<TxnIndex>>> {
        Arc::new(ModuleCode::from_deserialized(
            MockDeserializedCode::new(value),
            Arc::new(()),
            version,
        ))
    }

    pub(crate) fn mock_verified_code(
        value: usize,
        version: Option<TxnIndex>,
    ) -> Arc<ModuleCode<MockDeserializedCode, MockVerifiedCode, (), Option<TxnIndex>>> {
        Arc::new(ModuleCode::from_verified(
            MockVerifiedCode::new(value),
            Arc::new(()),
            version,
        ))
    }

    /// Returns a dummy [ModuleCode] in verified state.
    pub(crate) fn verified_code(
        module_name: &str,
        version: Option<TxnIndex>,
    ) -> Arc<ModuleCode<CompiledModule, Module, AptosModuleExtension, Option<TxnIndex>>> {
        let compiled_module = Arc::new(empty_module_with_dependencies_and_friends(
            module_name,
            vec![],
            vec![],
        ));
        let extension = Arc::new(AptosModuleExtension::new(StateValue::new_legacy(
            Bytes::new(),
        )));

        // The actual cintents of the module do not matter for tests, but we cannot mock it because
        // we have a static global cache for now.
        let dummy_runtime_environment = RuntimeEnvironment::new(vec![]);
        let locally_verified_module = dummy_runtime_environment
            .build_locally_verified_module(compiled_module, 0, &[0; 32])
            .unwrap();
        let verified_module = dummy_runtime_environment
            .build_verified_module(locally_verified_module, &[])
            .unwrap();

        Arc::new(ModuleCode::from_verified(
            verified_module,
            extension,
            version,
        ))
    }

    /// Returns a [ModuleId] for the given name.
    pub(crate) fn module_id(name: &str) -> ModuleId {
        ModuleId::new(AccountAddress::ONE, Identifier::new(name).unwrap())
    }
}
