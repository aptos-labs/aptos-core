// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::state_value::{StateValue, StateValueMetadata},
    write_set::{TransactionWrite, WriteOp},
};
use move_vm_types::types::Store;

#[derive(Clone, Debug)]
pub enum Op<T: Store> {
    Creation(T),
    CreationWithMetadata {
        data: T,
        metadata: StateValueMetadata,
    },
    Modification(T),
    ModificationWithMetadata {
        data: T,
        metadata: StateValueMetadata,
    },
    Deletion,
    DeletionWithMetadata {
        metadata: StateValueMetadata,
    },
}

impl<T: Store> Op<T> {
    #[inline]
    pub fn is_deletion(&self) -> bool {
        use Op::*;
        match self {
            Deletion | DeletionWithMetadata { .. } => true,
            Modification(_)
            | ModificationWithMetadata { .. }
            | Creation(_)
            | CreationWithMetadata { .. } => false,
        }
    }

    pub fn ok(&self) -> Option<&T> {
        use Op::*;
        match self {
            Creation(data)
            | Modification(data)
            | CreationWithMetadata { data, .. }
            | ModificationWithMetadata { data, .. } => Some(data),
            Deletion | DeletionWithMetadata { .. } => None,
        }
    }

    pub fn metadata(&self) -> Option<&StateValueMetadata> {
        use Op::*;
        match self {
            Creation(_) | Modification(_) | Deletion => None,
            CreationWithMetadata { metadata, .. }
            | ModificationWithMetadata { metadata, .. }
            | DeletionWithMetadata { metadata, .. } => Some(metadata),
        }
    }

    pub fn into_write_op(self) -> Option<WriteOp> {
        use Op::*;
        Some(match self {
            Creation(data) => WriteOp::Creation(data.into_bytes()?),
            CreationWithMetadata { data, metadata } => {
                let data = data.into_bytes()?;
                WriteOp::CreationWithMetadata { data, metadata }
            },
            Modification(data) => WriteOp::Modification(data.into_bytes()?),
            ModificationWithMetadata { data, metadata } => {
                let data = data.into_bytes()?;
                WriteOp::ModificationWithMetadata { data, metadata }
            },
            Deletion => WriteOp::Deletion,
            DeletionWithMetadata { metadata } => WriteOp::DeletionWithMetadata { metadata },
        })
    }

    pub fn squash(op: &mut Self, other: Self) -> anyhow::Result<bool> {
        use Op::*;
        match (&op, other) {
            (
                Modification(_)
                | ModificationWithMetadata { .. }
                | Creation(_)
                | CreationWithMetadata { .. },
                Creation(_) | CreationWithMetadata {..},
            ) // create existing
            | (
                Deletion | DeletionWithMetadata {..},
                Deletion | DeletionWithMetadata {..} | Modification(_) | ModificationWithMetadata { .. },
            ) // delete or modify already deleted
            => {
                anyhow::bail!(
                    "The given change sets cannot be squashed",
                )
            },
            (Modification(_), Modification(data) | ModificationWithMetadata {data, ..}) => *op = Modification(data),
            (ModificationWithMetadata{metadata, ..}, Modification(data) | ModificationWithMetadata{data, ..}) => {
                *op = ModificationWithMetadata{ data, metadata: metadata.clone()}
            },
            (Creation(_), Modification(data) | ModificationWithMetadata {data, ..} ) => {
                *op = Creation(data)
            },
            (CreationWithMetadata{metadata , ..}, Modification(data) | ModificationWithMetadata{data, ..}) => {
                *op = CreationWithMetadata{data, metadata: metadata.clone()}
            },
            (Modification(_) , Deletion | DeletionWithMetadata {..}) => {
                *op = Deletion
            },
            (ModificationWithMetadata{metadata, ..} , Deletion | DeletionWithMetadata {..}) => {
                *op = DeletionWithMetadata {metadata: metadata.clone()}
            },
            (Deletion, Creation(data) | CreationWithMetadata {data, ..}) => {
                *op = Modification(data)
            },
            (DeletionWithMetadata {metadata, ..}, Creation(data)| CreationWithMetadata {data, ..}) => {
                *op = ModificationWithMetadata{data, metadata: metadata.clone()}
            },
            (Creation(_) | CreationWithMetadata {..}, Deletion | DeletionWithMetadata {..}) => {
                return Ok(false)
            },
        }
        Ok(true)
    }
}

impl Op<Vec<u8>> {
    pub fn from_write_op(write_op: WriteOp) -> Self {
        match write_op {
            WriteOp::Creation(data) => Op::Creation(data),
            WriteOp::CreationWithMetadata { data, metadata } => {
                Op::CreationWithMetadata { data, metadata }
            },
            WriteOp::Modification(data) => Op::Modification(data),
            WriteOp::ModificationWithMetadata { data, metadata } => {
                Op::ModificationWithMetadata { data, metadata }
            },
            WriteOp::Deletion => Op::Deletion,
            WriteOp::DeletionWithMetadata { metadata } => Op::DeletionWithMetadata { metadata },
        }
    }
}

impl<T: Store> TransactionWrite for Op<T> {
    fn extract_raw_bytes(&self) -> Option<Vec<u8>> {
        // TODO: Seems like as_bytes() returns option on error, do we want this or should
        // it be a result?
        self.ok()?.as_bytes()
    }

    fn as_state_value(&self) -> Option<StateValue> {
        if let Some(data) = self.ok() {
            return Some(match self.metadata() {
                None => StateValue::new_legacy(data.as_bytes()?),
                Some(metadata) => StateValue::new_with_metadata(data.as_bytes()?, metadata.clone()),
            });
        }
        None
    }
}
