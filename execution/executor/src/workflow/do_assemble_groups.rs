// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Assembles resource-group blobs from the member changes transactions leave
//! pending in their write sets.

use crate::metrics::{ASSEMBLED_GROUPS, OTHER_TIMERS};
use anyhow::{bail, Result};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    state_store::{state_key::StateKey, TStateView},
    transaction::TransactionOutput,
    write_set::{PendingGroup, WriteOp},
};
use bytes::Bytes;
use move_core_types::language_storage::StructTag;
use rayon::prelude::*;
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap},
    sync::OnceLock,
};

/// A group's members, keyed by the BCS encoding of their struct tags.
/// Byte order on the keys is the order BCS writes map entries in, so encoding
/// is a concatenation.
type Members = BTreeMap<Bytes, Bytes>;

/// One group write to encode: the group as transaction `idx` left it.
struct Snapshot {
    idx: usize,
    group_key: StateKey,
    existed: bool,
    members: Members,
}

/// How the block-end pass is organized, chosen by the `GROUP_ASSEMBLY_PASS`
/// environment variable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssemblyPass {
    /// One serial walk over the outputs applies the changes and snapshots each
    /// written group; the snapshots are encoded in parallel. Unset selects
    /// this.
    Walk,
    /// A serial pass buckets the changes by group; each group then loads its
    /// base, replays its changes and encodes each version in its own parallel
    /// task.
    ByGroup,
}

impl AssemblyPass {
    pub fn get() -> Self {
        static PASS: OnceLock<AssemblyPass> = OnceLock::new();
        *PASS.get_or_init(|| match std::env::var("GROUP_ASSEMBLY_PASS").as_deref() {
            Err(_) | Ok("") | Ok("walk") => Self::Walk,
            Ok("by_group") => Self::ByGroup,
            Ok(other) => panic!("unknown GROUP_ASSEMBLY_PASS value {other:?}"),
        })
    }
}

pub struct DoAssembleGroups;

impl DoAssembleGroups {
    /// Replaces every output's pending group changes with group writes. The
    /// changes are applied in transaction order on top of the groups in
    /// `base`, the state before the block.
    pub fn run(
        outputs: &mut [TransactionOutput],
        base: &(impl TStateView<Key = StateKey> + Sync),
    ) -> Result<()> {
        if !outputs
            .iter()
            .any(|output| output.write_set().has_pending_groups())
        {
            return Ok(());
        }
        let _timer = OTHER_TIMERS.timer_with(&["assemble_groups"]);

        let writes = match AssemblyPass::get() {
            AssemblyPass::Walk => Self::walk(outputs, base)?,
            AssemblyPass::ByGroup => Self::by_group(outputs, base)?,
        };
        ASSEMBLED_GROUPS.inc_by(writes.len() as u64);

        for (idx, group_key, op) in writes {
            outputs[idx]
                .write_set_mut()
                .insert_value_write(group_key, op);
        }
        Ok(())
    }

    fn walk(
        outputs: &mut [TransactionOutput],
        base: &impl TStateView<Key = StateKey>,
    ) -> Result<Vec<(usize, StateKey, WriteOp)>> {
        let snapshots = {
            let _timer = OTHER_TIMERS.timer_with(&["assemble_groups__serial"]);
            let mut groups: HashMap<StateKey, Members> = HashMap::new();
            let mut snapshots = vec![];
            for (idx, output) in outputs.iter_mut().enumerate() {
                let Some(pending) = output.write_set_mut().take_pending_groups() else {
                    continue;
                };
                for (group_key, member_ops) in pending {
                    let members = match groups.entry(group_key.clone()) {
                        Entry::Occupied(entry) => entry.into_mut(),
                        Entry::Vacant(entry) => entry.insert(base_members(base, &group_key)?),
                    };
                    // A stored group is never empty.
                    let existed = !members.is_empty();
                    apply(members, member_ops)?;
                    snapshots.push(Snapshot {
                        idx,
                        group_key,
                        existed,
                        members: members.clone(),
                    });
                }
            }
            snapshots
        };

        let _timer = OTHER_TIMERS.timer_with(&["assemble_groups__parallel"]);
        snapshots
            .into_par_iter()
            .with_min_len(64)
            .map(|snapshot| {
                let op = group_op(snapshot.existed, &snapshot.members)?;
                Ok((snapshot.idx, snapshot.group_key, op))
            })
            .collect()
    }

    fn by_group(
        outputs: &mut [TransactionOutput],
        base: &(impl TStateView<Key = StateKey> + Sync),
    ) -> Result<Vec<(usize, StateKey, WriteOp)>> {
        let chains = {
            let _timer = OTHER_TIMERS.timer_with(&["assemble_groups__serial"]);
            let mut chains: HashMap<StateKey, Vec<(usize, PendingGroup)>> = HashMap::new();
            for (idx, output) in outputs.iter_mut().enumerate() {
                let Some(pending) = output.write_set_mut().take_pending_groups() else {
                    continue;
                };
                for (group_key, member_ops) in pending {
                    chains.entry(group_key).or_default().push((idx, member_ops));
                }
            }
            chains
        };

        let _timer = OTHER_TIMERS.timer_with(&["assemble_groups__parallel"]);
        let writes = chains
            .into_par_iter()
            .map(|(group_key, chain)| {
                let mut members = base_members(base, &group_key)?;
                let mut writes = Vec::with_capacity(chain.len());
                for (idx, member_ops) in chain {
                    // A stored group is never empty.
                    let existed = !members.is_empty();
                    apply(&mut members, member_ops)?;
                    writes.push((idx, group_key.clone(), group_op(existed, &members)?));
                }
                Ok(writes)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(writes.into_iter().flatten().collect())
    }
}

/// Applies one transaction's changes to a group's members.
fn apply(members: &mut Members, member_ops: PendingGroup) -> Result<()> {
    for (tag, op) in member_ops {
        let tag = Bytes::from(bcs::to_bytes(&tag)?);
        match op {
            Some(bytes) => {
                members.insert(tag, bytes);
            },
            None => {
                members.remove(&tag);
            },
        }
    }
    Ok(())
}

/// The members of the group stored at `group_key` before the block.
fn base_members(base: &impl TStateView<Key = StateKey>, group_key: &StateKey) -> Result<Members> {
    let Some(value) = base.get_state_value(group_key)? else {
        return Ok(Members::new());
    };
    bcs::from_bytes::<BTreeMap<StructTag, Bytes>>(value.bytes())?
        .into_iter()
        .map(|(tag, bytes)| Ok((Bytes::from(bcs::to_bytes(&tag)?), bytes)))
        .collect()
}

/// The write for a group's slot: creation, modification or deletion by
/// whether the group existed before the transaction and has members after.
fn group_op(existed: bool, members: &Members) -> Result<WriteOp> {
    Ok(match (existed, members.is_empty()) {
        (false, false) => WriteOp::legacy_creation(encode(members)),
        (true, false) => WriteOp::legacy_modification(encode(members)),
        (true, true) => WriteOp::legacy_deletion(),
        (false, true) => bail!("an empty group emitted a write"),
    })
}

/// BCS-encodes a group: the member count, then each entry's tag and
/// length-prefixed bytes in key order.
fn encode(members: &Members) -> Bytes {
    let len = uleb128_len(members.len())
        + members
            .iter()
            .map(|(tag, bytes)| tag.len() + uleb128_len(bytes.len()) + bytes.len())
            .sum::<usize>();
    let mut out = Vec::with_capacity(len);
    write_uleb128(&mut out, members.len());
    for (tag, bytes) in members {
        out.extend_from_slice(tag);
        write_uleb128(&mut out, bytes.len());
        out.extend_from_slice(bytes);
    }
    Bytes::from(out)
}

fn uleb128_len(mut value: usize) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    len
}

fn write_uleb128(out: &mut Vec<u8>, mut value: usize) {
    while value >= 0x80 {
        out.push((value as u8 & 0x7F) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

#[cfg(test)]
mod tests {
    use super::{encode, Members};
    use bytes::Bytes;
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::StructTag,
    };
    use std::collections::BTreeMap;

    fn tag(module: &str, name: &str) -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new(module).unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args: vec![],
        }
    }

    #[test]
    fn encode_matches_bcs() {
        // Tags of different lengths, so struct-tag order and byte order differ,
        // and a member long enough to need a two-byte length.
        let group = BTreeMap::from([
            (tag("object", "ObjectCore"), Bytes::from(vec![1u8; 40])),
            (
                tag("fungible_asset", "FungibleStore"),
                Bytes::from(vec![2u8; 200]),
            ),
            (tag("a", "B"), Bytes::from(vec![3u8; 1])),
            (tag("zzzzzzzzzzzzzzzzzzzz", "Z"), Bytes::new()),
        ]);
        let members = group
            .iter()
            .map(|(tag, bytes)| (Bytes::from(bcs::to_bytes(tag).unwrap()), bytes.clone()))
            .collect::<Members>();
        assert_eq!(encode(&members), bcs::to_bytes(&group).unwrap());
    }
}
