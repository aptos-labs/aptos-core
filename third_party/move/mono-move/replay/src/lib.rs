// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compares MonoMove against the legacy AptosVM on real chain transactions: captures them into a
//! corpus, replays each on both VMs on the same patched state, and reports a verdict per record.
//! See the `README`, the user guide in `docs/`, and `mono-move/docs/replay_comparison_design.md`.

/// The value of `$result`, or, on its error, a return of that error after the corpus being written
/// to `$writer` is finished with the records so far (see `CorpusWriter::finish_after`), while
/// `$what`: a run that stops for any reason keeps its work. Defined before the modules, so that
/// they see it.
macro_rules! or_stop {
    ($writer:ident, $result:expr, $what:expr) => {
        match $result {
            Ok(value) => value,
            Err(err) => return Err($writer.finish_after(err.into(), &$what)),
        }
    };
}

pub mod aggregate;
pub mod capture;
pub mod compare;
pub mod comparison;
pub mod completion;
pub mod corpus;
pub mod gas;
pub mod import;
pub mod isolated;
pub mod legacy;
pub mod overrides;
pub mod replay;
pub mod targets;
pub mod txn;
pub mod v1;
pub mod v2;

pub use mono_move_replay_common::panic_message;
