// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Smoke test of the REST chain source against live mainnet, at recent (unpruned) versions.
//! Needs network access, so it is ignored by default:
//!
//! ```bash
//! cargo test -p mono-move-replay --test rest_chain_source -- --ignored --nocapture
//! ```

use aptos_rest_client::{AptosBaseUrl, Client};
use mono_move_replay::import::{ChainSource, RestChainSource, PAGE_SIZE};

#[test]
#[ignore]
fn rest_chain_source_reads_recent_mainnet() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let latest = runtime
        .block_on(Client::new(AptosBaseUrl::Mainnet.to_url()).get_ledger_information())
        .expect("ledger info")
        .into_inner()
        .version;
    let start = (latest - 10_000) / PAGE_SIZE * PAGE_SIZE;

    let chain = RestChainSource::new(AptosBaseUrl::Mainnet, None, 2).expect("chain source");
    assert!(chain.identity().expect("identity").starts_with("chain-1-"));
    assert!(chain.latest_version().expect("latest version") >= latest);
    let ranges = chain
        .committed(&[(start, PAGE_SIZE), (start + PAGE_SIZE, 7)])
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()
        .expect("committed ranges");
    assert_eq!(ranges.len(), 2);
    for (range, (first, len)) in ranges
        .iter()
        .zip([(start, PAGE_SIZE), (start + PAGE_SIZE, 7)])
    {
        assert_eq!(range.len() as u64, len);
        assert_eq!(range[0].version, first);
    }
    let pages = ranges;
    let with_aux = pages
        .iter()
        .flatten()
        .filter(|c| c.aux_info != aptos_types::transaction::PersistedAuxiliaryInfo::None)
        .count();

    let framework = chain.framework_at(start).expect("framework");
    let addresses: std::collections::BTreeSet<_> = framework
        .keys()
        .filter_map(|k| match k.inner() {
            aptos_types::state_store::state_key::inner::StateKeyInner::AccessPath(ap) => {
                Some(ap.address)
            },
            aptos_types::state_store::state_key::inner::StateKeyInner::TableItem { .. }
            | aptos_types::state_store::state_key::inner::StateKeyInner::Raw(_)
            | aptos_types::state_store::state_key::inner::StateKeyInner::TradingNative(_) => None,
        })
        .collect();
    println!(
        "pages from {}: {} txns ({} with aux info); framework: {} modules at {:?}",
        start,
        pages.iter().map(Vec::len).sum::<usize>(),
        with_aux,
        framework.len(),
        addresses
    );
    assert!(framework.len() > 100);
    assert_eq!(addresses.len(), 3);
}
