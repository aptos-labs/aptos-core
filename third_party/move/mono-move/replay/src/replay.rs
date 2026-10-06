// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replays a record on one VM or both and prints each full output (status, gas, write set,
//! events), as the legacy comparison tool does when it runs a single VM. The record runs on the
//! same patched state `compare` uses.

use crate::{
    compare::op_kind_name,
    comparison::prepare,
    corpus::{Corpus, Shard, TxnRecord},
    isolated::{self, V2Limits, V2Outcome},
    overrides::PatchedState,
    panic_message,
    txn::{label, txn_kind},
    v1::{self, hit_metering_limit_on_chain},
};
use anyhow::Result;
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::TransactionOutput,
    write_set::{TransactionWrite, WriteOp},
};
use std::{collections::BTreeSet, io::Write};

/// Prints `record`'s output on V1 and/or V2 to `out`. V2 runs in a child process held to
/// `limits` (see [`isolated`]), so a MonoMove crash is printed rather than ending the replay.
pub fn replay(
    corpus: &Corpus,
    shard: &Shard,
    record: &TxnRecord,
    run_v1: bool,
    run_v2: bool,
    limits: &V2Limits,
    out: &mut dyn Write,
) -> Result<()> {
    writeln!(out, "version: {}", record.version)?;
    // A record whose state cannot be prepared is reported, so that the other records still run.
    let (input, state) = match prepare(corpus, shard, record) {
        Ok(prepared) => prepared,
        Err(err) => {
            writeln!(out, "could not prepare the state: {:#}", err)?;
            return Ok(());
        },
    };
    writeln!(
        out,
        "transaction: {} ({})",
        label(&record.txn),
        txn_kind(&record.txn)
    )?;
    if let Some(onchain) = &record.onchain {
        writeln!(
            out,
            "on chain: status {:?}, gas {}",
            onchain.status, onchain.gas_used
        )?;
    }
    writeln!(out, "overrides: {:?}", state.report().fired)?;
    // As in `compare`: replayed gas-free, a loop that gas stopped on chain could run for ever.
    if record
        .onchain
        .as_ref()
        .is_some_and(|onchain| hit_metering_limit_on_chain(&onchain.status))
    {
        writeln!(out, "not replayed: it stopped at a metering limit on chain")?;
        return Ok(());
    }
    out.flush()?;

    // Whether V1 stopped at a metering limit its status on chain does not show (see `V1Run`). V1
    // runs before MonoMove even when only MonoMove is asked for, unprinted, to tell.
    let mut v1_hit_metering_limit = false;
    if run_v1 || run_v2 {
        if run_v1 {
            writeln!(out, "\n== V1 (AptosVM)")?;
        }
        let view = state.view();
        let run = v1::run_caught(&view, &record.txn, record.aux_info);
        let reads = view.into_reads();
        // As in `compare`: only V1 on observed state is V1 on chain state.
        if let Ok(Ok(run)) = &run {
            v1_hit_metering_limit = run.hit_metering_limit && state.unobserved(&reads)?.is_empty();
        }
        if run_v1 {
            match run {
                Ok(Ok(run)) => print_output(out, &run.output)?,
                Ok(Err(err)) => writeln!(out, "error: {:#}", err)?,
                Err(panic) => writeln!(out, "panicked: {}", panic_message(&panic))?,
            }
            print_unobserved(out, &state, reads)?;
            out.flush()?;
        }
    }

    if run_v2 {
        writeln!(out, "\n== V2 (MonoMove)")?;
        if v1_hit_metering_limit {
            writeln!(
                out,
                "not run: V1 stopped at a metering limit, which unmetered MonoMove lacks"
            )?;
            return Ok(());
        }
        out.flush()?;
        let report = isolated::run_v2(&input, limits)?;
        match &report.outcome {
            V2Outcome::Ran(run) => {
                if let Some(unsupported) = &run.unsupported {
                    writeln!(out, "unsupported: {:?}", unsupported)?;
                }
                if let Some(status) = &run.output_gap_status {
                    writeln!(out, "status before the output gap: {:?}", status)?;
                }
                if let Some(vm_error) = &run.vm_error {
                    writeln!(out, "vm error: {}", vm_error)?;
                }
                match &run.output {
                    Ok(output) => print_output(out, output)?,
                    Err(err) => writeln!(out, "error: {}", err)?,
                }
            },
            V2Outcome::SetupFailed(err) => writeln!(out, "could not be set up: {}", err)?,
            V2Outcome::Panicked(message) => writeln!(out, "panicked: {}", message)?,
            V2Outcome::Crashed(reason) => writeln!(out, "crashed: {}", reason)?,
        }
        print_unobserved(out, &state, report.reads)?;
        out.flush()?;
    }
    Ok(())
}

fn print_output(out: &mut dyn Write, output: &TransactionOutput) -> Result<()> {
    writeln!(out, "status: {:?}", output.status())?;
    writeln!(out, "gas used: {}", output.gas_used())?;
    let writes: Vec<_> = output.write_set().write_op_iter().collect();
    writeln!(out, "write set: {} write(s)", writes.len())?;
    for (key, op) in writes {
        writeln!(out, "  {:?}", key)?;
        writeln!(out, "    {}", describe_write(op))?;
    }
    writeln!(out, "events: {}", output.events().len())?;
    for event in output.events() {
        writeln!(out, "  {}", describe_event(event))?;
    }
    Ok(())
}

fn describe_write(op: &WriteOp) -> String {
    let kind = op_kind_name(op.write_op_kind());
    match op.bytes() {
        Some(bytes) => format!(
            "{}, {} byte(s), metadata {:?}: 0x{}",
            kind,
            bytes.len(),
            op.metadata(),
            hex::encode(bytes)
        ),
        None => format!("{}, metadata {:?}", kind, op.metadata()),
    }
}

fn describe_event(event: &ContractEvent) -> String {
    let data = hex::encode(event.event_data());
    match event {
        ContractEvent::V1(v1) => format!(
            "v1 {} key {:?} seq {}: 0x{}",
            v1.type_tag().to_canonical_string(),
            v1.key(),
            v1.sequence_number(),
            data
        ),
        ContractEvent::V2(v2) => format!("v2 {}: 0x{}", v2.type_tag().to_canonical_string(), data),
    }
}

/// Lists the keys the VM read that the corpus holds no observation of: on them the output
/// above is not the chain's.
fn print_unobserved(
    out: &mut dyn Write,
    state: &PatchedState,
    reads: BTreeSet<StateKey>,
) -> Result<()> {
    let unobserved = state.unobserved(&reads)?;
    if !unobserved.is_empty() {
        writeln!(
            out,
            "warning: read {} key(s) the corpus never observed, so the output may differ from \
             the chain's:",
            unobserved.len()
        )?;
        for key in unobserved {
            writeln!(out, "  {:?}", key)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::{
        event::EventKey,
        transaction::{ExecutionStatus, TransactionAuxiliaryData, TransactionStatus},
        write_set::WriteSet,
    };
    use move_core_types::language_storage::TypeTag;
    use std::str::FromStr;

    fn event_type() -> TypeTag {
        TypeTag::from_str("0x1::test::Event").expect("type tag")
    }

    #[test]
    fn output_lists_every_write_and_event() {
        let key = StateKey::raw(b"k");
        let output = TransactionOutput::new(
            WriteSet::new(vec![
                (
                    key.clone(),
                    WriteOp::legacy_modification(vec![0xAB, 0xCD].into()),
                ),
                (StateKey::raw(b"z"), WriteOp::legacy_deletion()),
            ])
            .expect("write set"),
            vec![
                ContractEvent::new_v2(event_type(), vec![1, 2]).expect("v2 event"),
                ContractEvent::new_v1(EventKey::random(), 7, event_type(), vec![3])
                    .expect("v1 event"),
            ],
            42,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        );
        let mut printed = vec![];
        print_output(&mut printed, &output).expect("print");
        let printed = String::from_utf8(printed).expect("utf8");
        assert!(printed.contains("status: Keep(Success)"), "{printed}");
        assert!(printed.contains("gas used: 42"), "{printed}");
        assert!(printed.contains("write set: 2 write(s)"), "{printed}");
        assert!(printed.contains(&format!("{:?}", key)), "{printed}");
        assert!(printed.contains("modification, 2 byte(s)"), "{printed}");
        assert!(printed.contains(": 0xabcd"), "{printed}");
        assert!(printed.contains("deletion, metadata"), "{printed}");
        assert!(printed.contains("events: 2"), "{printed}");
        assert!(printed.contains("v2 0x1::test::Event: 0x0102"), "{printed}");
        assert!(printed.contains("seq 7: 0x03"), "{printed}");
    }
}
