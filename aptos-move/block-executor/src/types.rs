// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use serde::Serialize;
use std::{
    collections::HashSet,
    fmt::{self, Debug},
    hash::Hash,
};

/// The bounds the block executor requires of a resource group tag: keying the
/// multi-version group structures and write-set maps, and (bcs) serialization
/// when groups are sized and finalized. VMs choose the concrete representation
/// via [`Record::Tag`] and [`TransactionOutput::Tag`].
///
/// [`Record::Tag`]: crate::record::Record::Tag
/// [`TransactionOutput::Tag`]: crate::task::TransactionOutput::Tag
pub trait ResourceGroupTag:
    PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug + Serialize
{
}

impl<TG: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + Debug + Serialize> ResourceGroupTag
    for TG
{
}

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum InputOutputKey<K, T> {
    Resource(K),
    Group(K, T),
    DelayedField(DelayedFieldID),
}

pub struct ReadWriteSummary<K, TG> {
    pub reads: HashSet<InputOutputKey<K, TG>>,
    pub writes: HashSet<InputOutputKey<K, TG>>,
}

impl<K: Hash + Eq, TG: Hash + Eq> ReadWriteSummary<K, TG> {
    pub fn new(
        reads: HashSet<InputOutputKey<K, TG>>,
        writes: HashSet<InputOutputKey<K, TG>>,
    ) -> Self {
        Self { reads, writes }
    }

    pub fn conflicts_with_previous(&self, previous: &Self) -> bool {
        !self.reads.is_disjoint(&previous.writes)
    }

    pub fn find_conflicts<'a>(&'a self, previous: &'a Self) -> HashSet<&'a InputOutputKey<K, TG>> {
        self.reads
            .intersection(&previous.writes)
            .collect::<HashSet<_>>()
    }

    pub fn collapse_resource_group_conflicts(self) -> Self {
        let collapse = |k: InputOutputKey<K, TG>| match k {
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

impl<K: Debug, TG: Debug> fmt::Debug for ReadWriteSummary<K, TG> {
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
pub(crate) mod delayed_field_mock_serialization {
    use bytes::Bytes;
    use move_core_types::value::{MoveStructLayout, MoveTypeLayout};
    use move_vm_types::delayed_values::delayed_field_id::{DelayedFieldID, ExtractUniqueIndex};

    pub(crate) fn mock_layout() -> MoveTypeLayout {
        MoveTypeLayout::new_struct(MoveStructLayout::new(vec![]))
    }

    // ID is just the unique index as u128.
    pub(crate) fn serialize_from_delayed_field_u128(value_or_id: u128, version: u32) -> Bytes {
        let tuple = (value_or_id, version);
        serialize_delayed_field_tuple(&tuple)
    }

    pub(crate) fn serialize_from_delayed_field_id(
        delayed_field_id: DelayedFieldID,
        version: u32,
    ) -> Bytes {
        let tuple = (delayed_field_id.extract_unique_index() as u128, version);
        serialize_delayed_field_tuple(&tuple)
    }

    /// The width of the delayed field is not used in the tests, and fixed as 8 for
    /// all delayed field constructions. However, only the real ID is actually
    /// serialized and deserialized (together with the version).
    pub(crate) fn deserialize_to_delayed_field_u128(
        bytes: &[u8],
    ) -> Result<(u128, u32), bcs::Error> {
        bcs::from_bytes::<(u128, u32)>(bytes)
    }

    pub(crate) fn deserialize_to_delayed_field_id(
        bytes: &[u8],
    ) -> Result<(DelayedFieldID, u32), bcs::Error> {
        let (id, version) = bcs::from_bytes::<(u128, u32)>(bytes)?;
        Ok((DelayedFieldID::from((id as u32, 8)), version))
    }

    pub(crate) fn serialize_delayed_field_tuple(value: &(u128, u32)) -> Bytes {
        bcs::to_bytes(value)
            .expect("Failed to serialize (u128, u32) tuple")
            .into()
    }
}
