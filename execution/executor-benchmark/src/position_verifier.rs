// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks the published in-memory position index at every block boundary.
//!
//! After a block commits, its native-position ops are replayed into an
//! expected map, which both the block's own overlay from execution and
//! the published tip — each read over the base through the reader,
//! exactly as a consumer would — must enumerate to.

use aptos_db::InMemoryNativeStateReader;
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_storage_interface::state_store::positions::{
    position_key_of, NativeStateView, PositionKey,
};
use aptos_types::state_store::native_position::NativePosition;
use move_core_types::account_address::AccountAddress;
use std::collections::{BTreeMap, BTreeSet};

pub struct PositionIndexVerifier {
    reader: InMemoryNativeStateReader,
    expected: Mutex<BTreeMap<PositionKey, NativePosition>>,
    /// Exchanges whose positions from the checkpoint the run started on
    /// have been pulled into `expected`.
    seeded: Mutex<BTreeSet<AccountAddress>>,
}

impl std::fmt::Debug for PositionIndexVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PositionIndexVerifier")
    }
}

fn canon(p: &NativePosition) -> Vec<u8> {
    bcs::to_bytes(p).expect("serializable")
}

fn short(addr: &AccountAddress) -> String {
    let s = addr.to_hex();
    format!("..{}", &s[s.len() - 6..])
}

fn enumerate(
    view: &NativeStateView<'_>,
    exchanges: impl Iterator<Item = AccountAddress>,
) -> BTreeMap<PositionKey, NativePosition> {
    let mut index = BTreeMap::new();
    for exchange in exchanges {
        let accounts = view.for_each_account_in_exchange(exchange, |a, ps| Some((*a, ps.to_vec())));
        for (account, positions) in accounts {
            for (market, pos) in positions {
                let pk = PositionKey {
                    exchange: account.exchange,
                    account: account.account,
                    market,
                };
                assert!(index.insert(pk, pos).is_none(), "{pk:?}: enumerated twice");
            }
        }
    }
    index
}

impl PositionIndexVerifier {
    pub fn new(reader: InMemoryNativeStateReader) -> Self {
        Self {
            reader,
            expected: Mutex::new(BTreeMap::new()),
            seeded: Mutex::new(BTreeSet::new()),
        }
    }

    fn compare(
        what: &str,
        got: &BTreeMap<PositionKey, NativePosition>,
        want: &BTreeMap<PositionKey, NativePosition>,
        first: u64,
        last: u64,
    ) {
        assert_eq!(
            got.len(),
            want.len(),
            "block v{first}..v{last}: {what} enumerates {} positions, ledger implies {}",
            got.len(),
            want.len()
        );
        for (pk, want) in want {
            let got = got
                .get(pk)
                .unwrap_or_else(|| panic!("{pk:?}: missing from {what} at v{last}"));
            assert_eq!(
                canon(got),
                canon(want),
                "{pk:?}: {what} {got:?} != ledger {want:?} at v{last}"
            );
        }
    }

    /// Replays the block's ops into the expectation and checks the block's
    /// overlay and the published index against it. Panics on any
    /// disagreement.
    pub fn check_block(&self, execution_output: &ExecutionOutput) {
        let outputs = &execution_output.to_commit.transaction_outputs;
        if outputs.is_empty() {
            return;
        }
        let first = execution_output.first_version;
        let last = first + outputs.len() as u64 - 1;

        let ops: Vec<(PositionKey, Option<NativePosition>)> = outputs
            .iter()
            .flat_map(|out| out.write_set().native_position_iter())
            .map(|(key, op)| {
                let pk = position_key_of(key).expect("position key");
                let pos = op
                    .as_write_op()
                    .as_state_value_opt()
                    .map(|sv| NativePosition::deserialize(sv.bytes()).expect("decodable"));
                (pk, pos)
            })
            .collect();

        let mut expected = self.expected.lock();
        let mut seeded = self.seeded.lock();

        // An exchange first seen in this run: take what the checkpoint
        // already held for it. The index is read after this block's commit,
        // so re-applying the block's ops below is a no-op for it; real
        // checking of this exchange starts with the next block.
        let fresh: Vec<AccountAddress> = ops
            .iter()
            .map(|(pk, _)| pk.exchange)
            .filter(|e| seeded.insert(*e))
            .collect();
        if !fresh.is_empty() {
            let prior = self
                .reader
                .with_view(|view| enumerate(view, fresh.iter().copied()));
            info!(
                "[position-verify] seeding {} exchange(s) with {} positions found in the index at v{last}",
                fresh.len(),
                prior.len()
            );
            expected.extend(prior);
        }

        let mut written = Vec::new();
        for (pk, pos) in ops {
            match pos {
                Some(pos) => {
                    let overwrote = expected.insert(pk, pos).is_some();
                    written.push((pk, overwrote, false));
                },
                None => {
                    let existed = expected.remove(&pk).is_some();
                    written.push((pk, existed, true));
                },
            }
        }

        // The block's own overlay, as handed out by execution.
        let block_overlay = execution_output
            .positions
            .as_ref()
            .expect("execution output carries a position overlay");
        assert_eq!(block_overlay.version(), Some(last));
        let block_view = self.reader.with_view_of(block_overlay, |view| {
            enumerate(view, seeded.iter().copied())
        });
        Self::compare("block overlay", &block_view, &expected, first, last);

        // The published tip, after this block's commit.
        let (base_version, index) = self
            .reader
            .with_view(|view| (view.base_version(), enumerate(view, seeded.iter().copied())));
        let tip_version = self.reader.tip_version();
        Self::compare("tip", &index, &expected, first, last);

        info!(
            "[position-verify] block v{first}..v{last} ({} ops): block overlay == tip == ledger. {} live positions; base@{:?}, tip@{:?} ({} versions in overlay)",
            written.len(),
            expected.len(),
            base_version,
            tip_version,
            tip_version.unwrap_or(0).saturating_sub(base_version.unwrap_or(0)),
        );
        // A few concrete entries: the last writes of this block, read back
        // from the index, next to what the ledger says.
        let mut shown = BTreeSet::new();
        for (pk, prior, deleted) in written.iter().rev() {
            if !shown.insert(*pk) {
                continue;
            }
            let from_index = index.get(pk).map(|p| p.size());
            let from_ledger = expected.get(pk).map(|p| p.size());
            info!(
                "[position-verify]   acct {} mkt {}: index={:?} ledger={:?}{}",
                short(&pk.account),
                short(&pk.market),
                from_index,
                from_ledger,
                match (deleted, prior) {
                    (true, true) => " (deleted this block)",
                    (true, false) => " (delete of absent key)",
                    (false, true) => " (overwrote an earlier write)",
                    (false, false) => " (new this block)",
                }
            );
            if shown.len() == 3 {
                break;
            }
        }
    }
}
