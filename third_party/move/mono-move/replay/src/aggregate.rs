// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Merges the results of several `compare` runs, e.g. one per shard, into one summary and report.

use crate::comparison::{RecordResult, Summary, UnsupportedKind};
use anyhow::{bail, Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

/// Reads the results in `paths`: files `compare` wrote, or directories of `*.jsonl` files.
/// Returns the files read, sorted and each once, and their results in that order. A record of
/// one corpus appearing twice is an error, since it would be counted twice; the same version from
/// different corpora is two records.
pub fn read_all_results(paths: &[PathBuf]) -> Result<(Vec<PathBuf>, Vec<RecordResult>)> {
    let mut files = BTreeSet::new();
    let mut add = |file: &Path| -> Result<()> {
        files.insert(
            file.canonicalize()
                .with_context(|| format!("failed to resolve {:?}", file))?,
        );
        Ok(())
    };
    for path in paths {
        if path.is_dir() {
            for entry in
                std::fs::read_dir(path).with_context(|| format!("failed to list {:?}", path))?
            {
                let file = entry?.path();
                if file.extension().is_some_and(|ext| ext == "jsonl") {
                    add(&file)?;
                }
            }
        } else {
            add(path)?;
        }
    }
    let mut results = vec![];
    let mut seen = BTreeMap::new();
    for file in &files {
        for result in read_results(file)? {
            if let Some(first) = seen.insert((result.corpus_id.clone(), result.version), file) {
                bail!(
                    "version {} of corpus {:?} appears in both {:?} and {:?}",
                    result.version,
                    result.corpus_id,
                    first,
                    file
                );
            }
            results.push(result);
        }
    }
    Ok((files.into_iter().collect(), results))
}

/// Reads the JSON lines `compare` wrote.
pub fn read_results(path: &Path) -> Result<Vec<RecordResult>> {
    let file = std::fs::File::open(path).with_context(|| format!("failed to open {:?}", path))?;
    let mut results = vec![];
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| format!("failed to read {:?}", path))?;
        if line.trim().is_empty() {
            continue;
        }
        results.push(serde_json::from_str(&line).with_context(|| {
            format!(
                "{:?} line {} is not a compare result (results written before corpus ids \
                         were recorded need compare to be rerun)",
                path,
                i + 1
            )
        })?);
    }
    Ok(results)
}

/// Why the results fail: no record was compared, or MonoMove mismatched. Empty if they pass.
pub fn failures(summary: &Summary) -> Vec<String> {
    let mut failures = vec![];
    if summary.records == 0 {
        failures.push("no records were compared".to_string());
    } else if [
        summary.matches,
        summary.known_differences,
        summary.mismatches,
    ]
    .iter()
    .all(|count| *count == 0)
    {
        // Every record was withheld (unsupported, incomplete, not a candidate, ...): nothing
        // backs a verdict on the outputs.
        failures.push("no record reached the output comparison".to_string());
    }
    if summary.mismatches > 0 {
        failures.push(format!("{} mismatch(es)", summary.mismatches));
    }
    if summary.mono_crashes > 0 {
        failures.push(format!("{} MonoMove crash(es)", summary.mono_crashes));
    }
    failures
}

/// A Markdown report of the summary.
pub fn markdown(summary: &Summary, failures: &[String]) -> String {
    let mut out = String::new();
    let verdict = if failures.is_empty() {
        "passed"
    } else {
        "failed"
    };
    let _ = writeln!(out, "## MonoMove replay comparison: {}\n", verdict);
    for failure in failures {
        let _ = writeln!(out, "- **{}**", failure);
    }
    if !failures.is_empty() {
        out.push('\n');
    }
    if summary.versions_in_several_corpora > 0 {
        let _ = writeln!(
            out,
            "Note: {} version(s) appear in more than one corpus and are counted once per corpus.\n",
            summary.versions_in_several_corpora
        );
    }
    let _ = writeln!(out, "| Outcome | Records |\n|---|---|");
    let incomplete: usize = summary.incomplete.values().sum();
    for (name, count) in [
        ("match", summary.matches),
        ("known difference", summary.known_differences),
        ("mismatch", summary.mismatches),
        ("MonoMove unsupported", summary.unsupported),
        ("MonoMove crash", summary.mono_crashes),
        ("incomplete", incomplete),
        ("not candidate", summary.not_candidates),
        ("not comparable", summary.not_comparable),
        ("V1 error", summary.v1_errors),
        ("skipped", summary.skipped),
        ("total", summary.records),
    ] {
        let _ = writeln!(out, "| {} | {} |", name, count);
    }
    if !summary.unsupported_kinds.is_empty() {
        let _ = writeln!(
            out,
            "\n### MonoMove gaps by frequency\n\n| Gap | Records |\n|---|---|"
        );
        for (kind, count) in &summary.unsupported_kinds {
            let _ = writeln!(out, "| {} | {} |", describe_gap(kind), count);
        }
    }
    out
}

fn describe_gap(kind: &UnsupportedKind) -> String {
    match kind {
        UnsupportedKind::MissingNative { module, function } => {
            format!("missing native `{}::{}`", module, function)
        },
        UnsupportedKind::LoweringSkipped { reason } => format!("lowering skipped: {}", reason),
        UnsupportedKind::ResourceLayoutNotDerivable => "resource layout not derivable".to_string(),
        UnsupportedKind::RuntimeUnsupported { what } => format!("unsupported: {}", what),
        UnsupportedKind::TransactionShape { what } => format!("transaction shape: {}", what),
        UnsupportedKind::Other { detail } => format!("other: {}", detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparison::Outcome;
    use aptos_types::transaction::Version;

    fn result(outcome: Outcome) -> RecordResult {
        RecordResult {
            corpus_id: "c".to_string(),
            version: 1,
            label: "f".to_string(),
            overrides: vec!["zero_gas".to_string()],
            onchain_checked: true,
            outcome,
        }
    }

    #[test]
    fn results_round_trip_through_json_lines() {
        let results = [
            result(Outcome::Match),
            result(Outcome::Incomplete {
                vm: "v2".to_string(),
                unobserved: vec!["k".to_string()],
            }),
            result(Outcome::MonoUnsupported {
                kind: UnsupportedKind::MissingNative {
                    module: "1::m".to_string(),
                    function: "f".to_string(),
                },
                v1_status: "Keep(Success)".to_string(),
            }),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, mut r)| {
            r.version = i as Version;
            r
        })
        .collect::<Vec<_>>();
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("results.jsonl");
        write_results(&path, &results);
        // The file named twice, directly and through its directory, is read once.
        let (files, read) =
            read_all_results(&[dir.path().to_path_buf(), path.clone()]).expect("read");
        assert_eq!(files, vec![path.canonicalize().expect("canonical")]);
        assert_eq!(
            read.iter().map(|r| r.outcome.clone()).collect::<Vec<_>>(),
            results
                .iter()
                .map(|r| r.outcome.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_record_in_two_files_is_an_error_but_a_version_of_two_corpora_is_not() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_results(&dir.path().join("a.jsonl"), &[result(Outcome::Match)]);
        let mut other = result(Outcome::Match);
        other.corpus_id = "d".to_string();
        write_results(&dir.path().join("b.jsonl"), &[other]);
        let (_, read) = read_all_results(&[dir.path().to_path_buf()]).expect("two corpora");
        assert_eq!(read.len(), 2);
        // Counted once per corpus, and said so.
        let summary = Summary::new(&read);
        assert_eq!(summary.versions_in_several_corpora, 1);
        assert!(markdown(&summary, &[]).contains("1 version(s) appear in more than one corpus"));

        write_results(&dir.path().join("c.jsonl"), &[result(Outcome::Match)]);
        let err = read_all_results(&[dir.path().to_path_buf()]).expect_err("duplicate");
        assert!(
            err.to_string()
                .contains("version 1 of corpus \"c\" appears in both"),
            "{err}"
        );
    }

    fn write_results(path: &Path, results: &[RecordResult]) {
        let lines: Vec<String> = results
            .iter()
            .map(|r| serde_json::to_string(r).expect("json"))
            .collect();
        std::fs::write(path, lines.join("\n")).expect("write");
    }

    #[test]
    fn mismatches_and_empty_results_fail() {
        let summary = |outcomes: Vec<Outcome>| {
            Summary::new(&outcomes.into_iter().map(result).collect::<Vec<_>>())
        };
        assert!(
            failures(&summary(vec![Outcome::Match, Outcome::KnownDifference {
                diffs: vec![]
            }]))
            .is_empty()
        );
        assert_eq!(
            failures(&summary(vec![Outcome::Match, Outcome::Mismatch {
                diffs: vec![]
            }])),
            vec!["1 mismatch(es)".to_string()]
        );
        assert_eq!(failures(&summary(vec![])), vec![
            "no records were compared".to_string()
        ]);
        assert_eq!(
            failures(&summary(vec![Outcome::Skipped {
                reason: String::new()
            }])),
            vec!["no record reached the output comparison".to_string()]
        );
    }
}
