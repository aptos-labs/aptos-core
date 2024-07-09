use crate::{
    state_store::state_value::{StateValue, StateValueMetadata},
    write_set::{WriteOp, WriteOpSize},
};
use bytes::Bytes;
use move_binary_format::{
    access::ModuleAccess,
    deserializer::DeserializerConfig,
    errors::{PartialVMError, PartialVMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, metadata::Metadata,
    vm_status::StatusCode,
};
use std::sync::Arc;

/// In addition to a basic write op information, carries non-serialized module.
/// Note that this is not at all redundant: when a module is published, it is
/// deserialized, and so we can capture it here for later caching.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ModuleWriteOp {
    // Modules cannot be deleted, so they can be:
    //   - created, or
    //   - modified (upgraded).
    is_creation: bool,
    // Metadata in order to create a write op which is persisted later in storage.
    state_value_metadata: StateValueMetadata,
    // Serialized and non-serialized representations.
    module: Arc<CompiledModule>,
    serialized_module_bytes: Bytes,
}

impl ModuleWriteOp {
    fn new(
        is_creation: bool,
        state_value_metadata: StateValueMetadata,
        module: CompiledModule,
        serialized_module_bytes: Bytes,
    ) -> Self {
        let module = Arc::new(module);
        Self {
            is_creation,
            state_value_metadata,
            module,
            serialized_module_bytes,
        }
    }

    pub fn creation(
        state_value_metadata: StateValueMetadata,
        module: CompiledModule,
        serialized_module_bytes: Bytes,
    ) -> Self {
        Self::new(true, state_value_metadata, module, serialized_module_bytes)
    }

    pub fn legacy_creation(module: CompiledModule, serialized_module_bytes: Bytes) -> Self {
        Self::new(
            true,
            StateValueMetadata::none(),
            module,
            serialized_module_bytes,
        )
    }

    pub fn modification(
        state_value_metadata: StateValueMetadata,
        module: CompiledModule,
        serialized_module_bytes: Bytes,
    ) -> Self {
        Self::new(false, state_value_metadata, module, serialized_module_bytes)
    }

    pub fn legacy_modification(module: CompiledModule, serialized_module_bytes: Bytes) -> Self {
        Self::new(
            false,
            StateValueMetadata::none(),
            module,
            serialized_module_bytes,
        )
    }

    /// Tries to construct a module write from its storage representation. Fails if:
    ///   1) storage write is a deletion (invariant violation), or
    ///   2) module deserialization fails.
    pub fn from_write_op(
        write_op: WriteOp,
        deserializer_config: &DeserializerConfig,
    ) -> PartialVMResult<Self> {
        use crate::write_set::WriteOp::*;
        let (is_creation, state_value_metadata, serialized_module_bytes) = match write_op {
            Creation { data, metadata } => (true, metadata, data),
            Modification { data, metadata } => (false, metadata, data),
            Deletion { .. } => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("Module write cannot be a deletion".to_string()),
                )
            },
        };

        let module =
            CompiledModule::deserialize_with_config(&serialized_module_bytes, deserializer_config)?;
        Ok(Self::new(
            is_creation,
            state_value_metadata,
            module,
            serialized_module_bytes,
        ))
    }

    pub fn compiled_module(&self) -> &CompiledModule {
        self.module.as_ref()
    }

    pub fn module_bytes(&self) -> &Bytes {
        &self.serialized_module_bytes
    }

    pub fn write_op_size(&self) -> WriteOpSize {
        let write_len = self.serialized_module_bytes.len() as u64;
        if self.is_creation {
            WriteOpSize::Creation { write_len }
        } else {
            WriteOpSize::Modification { write_len }
        }
    }

    pub fn get_metadata_mut(&mut self) -> &mut StateValueMetadata {
        &mut self.state_value_metadata
    }

    pub fn into_write_op(self) -> WriteOp {
        let data = self.serialized_module_bytes;
        let metadata = self.state_value_metadata;
        if self.is_creation {
            WriteOp::Creation { data, metadata }
        } else {
            WriteOp::Modification { data, metadata }
        }
    }
}

pub trait ModuleWrite: Sized {
    fn serialized_module_bytes(&self) -> &Bytes;

    fn module_size_in_bytes(&self) -> usize {
        self.serialized_module_bytes().len()
    }

    fn module_state_value_metadata(&self) -> StateValueMetadata;

    fn module_metadata(&self) -> Vec<Metadata>;

    fn immediate_dependencies(&self) -> Vec<(AccountAddress, Identifier)>;

    fn immediate_friends(&self) -> Vec<(AccountAddress, Identifier)>;

    fn from_state_value(
        state_value: StateValue,
        deserializer_config: &DeserializerConfig,
    ) -> PartialVMResult<Self>;
}

impl ModuleWrite for ModuleWriteOp {
    fn serialized_module_bytes(&self) -> &Bytes {
        &self.serialized_module_bytes
    }

    fn module_state_value_metadata(&self) -> StateValueMetadata {
        self.state_value_metadata.clone()
    }

    fn module_metadata(&self) -> Vec<Metadata> {
        self.compiled_module().metadata.clone()
    }

    fn immediate_dependencies(&self) -> Vec<(AccountAddress, Identifier)> {
        self.compiled_module()
            .immediate_dependencies()
            .into_iter()
            .map(|module_id| (*module_id.address(), module_id.name().to_owned()))
            .collect()
    }

    fn immediate_friends(&self) -> Vec<(AccountAddress, Identifier)> {
        self.compiled_module()
            .immediate_friends()
            .into_iter()
            .map(|module_id| (*module_id.address(), module_id.name().to_owned()))
            .collect()
    }

    fn from_state_value(
        state_value: StateValue,
        deserializer_config: &DeserializerConfig,
    ) -> PartialVMResult<Self> {
        let (state_value_metadata, module_bytes) = state_value.unpack();
        let compiled_module =
            CompiledModule::deserialize_with_config(&module_bytes, deserializer_config)?;

        // Because state value exists, we treat it as modification.
        Ok(Self::modification(
            state_value_metadata,
            compiled_module,
            module_bytes,
        ))
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_module_write_op_creation() {
        // FIXME(George): Ensure this is tested.
    }
}
