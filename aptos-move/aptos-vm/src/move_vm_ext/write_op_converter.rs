// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{session::BytesWithResourceLayout, AptosMoveResolver};
use aptos_aggregator::delta_change_set::serialize;
use aptos_types::{
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    state_store::{
        state_key::StateKey,
        state_value::{StateValueMetadata, StateValueMetadataKind},
    },
    write_set::WriteOp,
};
use bytes::Bytes;
use move_core_types::{
    effects::Op as MoveStorageOp,
    value::MoveTypeLayout,
    vm_status::{err_msg, StatusCode, VMStatus},
};
use std::sync::Arc;

pub(crate) struct WriteOpConverter<'r> {
    remote: &'r dyn AptosMoveResolver,
    new_slot_metadata: Option<StateValueMetadata>,
}

macro_rules! convert_impl {
    ($convert_func_name:ident, $get_metadata_callback:ident) => {
        pub(crate) fn $convert_func_name(
            &self,
            state_key: &StateKey,
            move_storage_op: MoveStorageOp<Bytes>,
            legacy_creation_as_modification: bool,
        ) -> Result<WriteOp, VMStatus> {
            let move_storage_op = match move_storage_op {
                MoveStorageOp::New(data) => MoveStorageOp::New((data, None)),
                MoveStorageOp::Modify(data) => MoveStorageOp::Modify((data, None)),
                MoveStorageOp::Delete => MoveStorageOp::Delete,
            };
            self.convert(
                self.remote.$get_metadata_callback(state_key),
                move_storage_op,
                legacy_creation_as_modification,
            )
        }
    };
}

impl<'r> WriteOpConverter<'r> {
    convert_impl!(convert_module, get_module_state_value_metadata);

    convert_impl!(convert_aggregator, get_aggregator_v1_state_value_metadata);

    pub(crate) fn convert_resource(
        &self,
        state_key: &StateKey,
        move_storage_op: MoveStorageOp<BytesWithResourceLayout>,
        legacy_creation_as_modification: bool,
    ) -> Result<(WriteOp, Option<Arc<MoveTypeLayout>>), VMStatus> {
        let result = self.convert(
            self.remote.get_resource_state_value_metadata(state_key),
            move_storage_op.clone(),
            legacy_creation_as_modification,
        );
        match move_storage_op {
            MoveStorageOp::New((_, type_layout)) => Ok((result?, type_layout)),
            MoveStorageOp::Modify((_, type_layout)) => Ok((result?, type_layout)),
            MoveStorageOp::Delete => Ok((result?, None)),
        }
    }

    pub(crate) fn new(
        remote: &'r dyn AptosMoveResolver,
        is_storage_slot_metadata_enabled: bool,
    ) -> Self {
        let mut new_slot_metadata: Option<StateValueMetadata> = None;
        if is_storage_slot_metadata_enabled {
            if let Some(current_time) = CurrentTimeMicroseconds::fetch_config(remote) {
                // The deposit on the metadata is a placeholder (0), it will be updated later when
                // storage fee is charged.
                new_slot_metadata = Some(StateValueMetadata::new(0, &current_time));
            }
        }

        Self {
            remote,
            new_slot_metadata,
        }
    }

    fn convert(
        &self,
        state_value_metadata_result: anyhow::Result<Option<StateValueMetadataKind>>,
        move_storage_op: MoveStorageOp<BytesWithResourceLayout>,
        legacy_creation_as_modification: bool,
    ) -> Result<WriteOp, VMStatus> {
        use MoveStorageOp::*;
        use WriteOp::*;

        let maybe_existing_metadata = state_value_metadata_result.map_err(|_| {
            VMStatus::error(
                StatusCode::DELAYED_FIELDS_SPECULATIVE_ABORT_ERROR,
                err_msg("Storage read failed when converting change set."),
            )
        })?;

        let write_op = match (maybe_existing_metadata, move_storage_op) {
            (None, Modify(_) | Delete) => {
                return Err(VMStatus::error(
                    // Possible under speculative execution, returning speculative error waiting for re-execution
                    StatusCode::DELAYED_FIELDS_SPECULATIVE_ABORT_ERROR,
                    err_msg("When converting write op: updating non-existent value."),
                ));
            },
            (Some(_), New(_)) => {
                return Err(VMStatus::error(
                    // Possible under speculative execution, returning speculative error waiting for re-execution
                    StatusCode::DELAYED_FIELDS_SPECULATIVE_ABORT_ERROR,
                    err_msg("When converting write op: Recreating existing value."),
                ));
            },
            (None, New((data, _))) => match &self.new_slot_metadata {
                None => {
                    if legacy_creation_as_modification {
                        Modification(data)
                    } else {
                        Creation(data)
                    }
                },
                Some(metadata) => CreationWithMetadata {
                    data,
                    metadata: metadata.clone(),
                },
            },
            (Some(existing_metadata), Modify((data, _))) => {
                // Inherit metadata even if the feature flags is turned off, for compatibility.
                match existing_metadata {
                    None => Modification(data),
                    Some(metadata) => ModificationWithMetadata { data, metadata },
                }
            },
            (Some(existing_metadata), Delete) => {
                // Inherit metadata even if the feature flags is turned off, for compatibility.
                match existing_metadata {
                    None => Deletion,
                    Some(metadata) => DeletionWithMetadata { metadata },
                }
            },
        };
        Ok(write_op)
    }

    pub(crate) fn convert_aggregator_modification(
        &self,
        state_key: &StateKey,
        value: u128,
    ) -> Result<WriteOp, VMStatus> {
        let maybe_existing_metadata = self
            .remote
            .get_aggregator_v1_state_value_metadata(state_key)
            .map_err(|_| {
                VMStatus::error(StatusCode::DELAYED_FIELDS_SPECULATIVE_ABORT_ERROR, None)
            })?;
        let data = serialize(&value).into();

        let op = match maybe_existing_metadata {
            None => {
                match &self.new_slot_metadata {
                    // n.b. Aggregator writes historically did not distinguish Create vs Modify.
                    None => WriteOp::Modification(data),
                    Some(metadata) => WriteOp::CreationWithMetadata {
                        data,
                        metadata: metadata.clone(),
                    },
                }
            },
            Some(existing_metadata) => match existing_metadata {
                None => WriteOp::Modification(data),
                Some(metadata) => WriteOp::ModificationWithMetadata { data, metadata },
            },
        };

        Ok(op)
    }
}
