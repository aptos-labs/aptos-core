// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts a real legacy dump into a corpus and checks that every record reads back unchanged.
//! Needs an unpacked dump, so it is ignored by default:
//!
//! ```bash
//! LEGACY_DUMP_DIR=<dir> cargo test -p mono-move-replay --test legacy_round_trip \
//!     -- --ignored --nocapture
//! ```

use aptos_types::transaction::PersistedAuxiliaryInfo;
use mono_move_replay::{
    corpus::{Corpus, CorpusWriter, FrameworkSource, Origin, RecordInput},
    legacy::LegacyDump,
};
use std::collections::BTreeMap;

#[test]
#[ignore]
fn legacy_dump_round_trips_through_a_corpus() {
    let dump_dir = std::env::var("LEGACY_DUMP_DIR").expect("set LEGACY_DUMP_DIR");
    let dump = LegacyDump::open(&dump_dir).expect("open dump");
    let era = dump.era().expect("era");
    let versions = dump.versions().expect("versions");
    assert!(!versions.is_empty(), "the dump holds no records");

    let out = tempfile::tempdir().expect("tempdir");
    let mut writer = CorpusWriter::create(
        out.path(),
        "round-trip",
        "mainnet",
        Origin::LegacyDump {
            source: dump_dir.clone(),
            era,
            framework: FrameworkSource::Head,
        },
        500,
    )
    .expect("create writer");
    for version in &versions {
        let record = dump.read(*version).expect("read legacy record");
        writer
            .add(RecordInput {
                version: record.version,
                txn: record.txn,
                aux_info: PersistedAuxiliaryInfo::None,
                state: record.state,
                framework: None,
                absent: Default::default(),
                onchain: None,
            })
            .expect("add record");
    }
    let manifest = writer.finish().expect("finish");
    assert_eq!(manifest.num_records(), versions.len());

    let dumped_bytes: u64 = std::fs::read_dir(std::path::Path::new(&dump_dir).join("state_data"))
        .expect("list state files")
        .map(|e| e.expect("entry").metadata().expect("metadata").len())
        .sum();
    let corpus_bytes: u64 = std::fs::read_dir(out.path().join("shards"))
        .expect("list shards")
        .map(|e| e.expect("entry").metadata().expect("metadata").len())
        .sum::<u64>()
        + std::fs::metadata(out.path().join("modules.pack"))
            .expect("modules")
            .len();
    println!(
        "{} records, {} shards, {} modules: state files {} KB -> corpus {} KB",
        manifest.num_records(),
        manifest.shards.len(),
        manifest.num_modules,
        dumped_bytes / 1024,
        corpus_bytes / 1024
    );

    let corpus = Corpus::open(out.path()).expect("open corpus");
    let mut checked = 0;
    for n in 0..manifest.shards.len() {
        let shard = corpus.load_shard(n).expect("load shard");
        for record in &shard.records {
            let original = dump.read(record.version).expect("re-read legacy record");
            let state: BTreeMap<_, _> = corpus
                .record_state(&shard, record)
                .expect("resolve state")
                .into_iter()
                .collect();
            assert_eq!(state, original.state, "state of {}", record.version);
            assert_eq!(record.txn, original.txn, "txn of {}", record.version);
            checked += 1;
        }
    }
    assert_eq!(checked, versions.len());
}
