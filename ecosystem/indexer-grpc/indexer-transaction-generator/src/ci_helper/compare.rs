// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Comparison of generated transactions with the checked-in baseline.

use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    pub diff_found: bool,
    pub new_file_found: bool,
}

impl Comparison {
    /// The processor repository is dispatched only for changed transactions.
    /// A new transaction needs manual review first.
    pub fn dispatch_required(&self) -> bool {
        self.diff_found && !self.new_file_found
    }

    /// The step outputs, in `$GITHUB_OUTPUT` format.
    pub fn github_outputs(&self) -> String {
        format!(
            "diff_found={}\nnew_file_found={}\ndispatch_required={}\n",
            self.diff_found,
            self.new_file_found,
            self.dispatch_required()
        )
    }
}

/// Compares each generated `*.json` file with the file at the same relative path in
/// `baseline_dir`. Imported transactions are compared byte for byte. Scripted transactions
/// come from a fresh local node, so both sides are normalized first.
pub fn compare(baseline_dir: &Path, generated_dir: &Path) -> Result<Comparison> {
    ensure!(
        baseline_dir.is_dir(),
        "baseline directory does not exist: {}",
        baseline_dir.display()
    );
    ensure!(
        generated_dir.is_dir(),
        "generated directory does not exist: {}",
        generated_dir.display()
    );
    let mut relative_paths = Vec::new();
    json_files(generated_dir, generated_dir, &mut relative_paths)?;
    let mut comparison = Comparison::default();
    for relative in relative_paths {
        let read: fn(&Path) -> Result<Vec<u8>> = if is_scripted(&relative)? {
            normalized_file
        } else {
            raw_file
        };
        let generated = read(&generated_dir.join(&relative))?;
        let baseline = baseline_dir.join(&relative);
        if !baseline.exists() {
            comparison.new_file_found = true;
            println!("New generated transaction: {}", relative.display());
        } else if read(&baseline)? != generated {
            comparison.diff_found = true;
            println!("Generated transaction differs: {}", relative.display());
        }
    }
    Ok(comparison)
}

/// Returns true for `scripted_transactions/<name>.json` and false for `imported_*/...`.
fn is_scripted(relative: &Path) -> Result<bool> {
    let components: Vec<String> = relative
        .iter()
        .map(|component| component.to_string_lossy().into_owned())
        .collect();
    match components.as_slice() {
        [first, ..] if first.starts_with("imported_") => Ok(false),
        [first, _] if first == "scripted_transactions" => Ok(true),
        _ => bail!(
            "generated JSON has an unexpected path: {}",
            relative.display()
        ),
    }
}

fn json_files(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory {}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            json_files(root, &path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn raw_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("failed to read {}", path.display()))
}

fn normalized_file(path: &Path) -> Result<Vec<u8>> {
    let mut transaction: Value = serde_json::from_slice(&raw_file(path)?)
        .with_context(|| format!("transaction at {} is not valid JSON", path.display()))?;
    normalize_transaction(&mut transaction)?;
    Ok(serde_json::to_vec_pretty(&transaction)?)
}

/// Fields that change between two runs of the same script on a fresh local node.
const DYNAMIC_FIELDS: [&str; 13] = [
    "/timestamp",
    "/version",
    "/epoch",
    "/blockHeight",
    "/sizeInfo",
    "/info/hash",
    "/info/stateChangeHash",
    "/info/accumulatorRootHash",
    "/user/request/sender",
    "/user/request/expirationTimestampSecs/seconds",
    "/user/request/signature/ed25519/publicKey",
    "/user/request/signature/ed25519/signature",
    "/user/request/payload/scriptPayload/code/bytecode",
];

/// Dynamic fields inside each element of `/info/changes`.
const DYNAMIC_CHANGE_FIELDS: [&str; 6] = [
    "/writeResource/stateKeyHash",
    "/writeResource/address",
    "/writeResource/type/address",
    "/writeTableItem/stateKeyHash",
    "/writeTableItem/data/key",
    "/writeTableItem/data/value",
];

fn normalize_transaction(transaction: &mut Value) -> Result<()> {
    for pointer in DYNAMIC_FIELDS {
        remove_pointer(transaction, pointer);
    }
    let changes = transaction
        .pointer_mut("/info/changes")
        .and_then(Value::as_array_mut)
        .context("transaction info changes must be an array")?;
    for change in changes {
        for pointer in DYNAMIC_CHANGE_FIELDS {
            remove_pointer(change, pointer);
        }
        if let Some(Value::String(data)) = change.pointer_mut("/writeResource/data") {
            let mut embedded: Value = serde_json::from_str(data)
                .context("writeResource data is not valid embedded JSON")?;
            if let Some(object) = embedded.as_object_mut() {
                object.remove("authentication_key");
            }
            remove_key_recursively(&mut embedded, "addr");
            *data = serde_json::to_string(&embedded)?;
        }
    }
    Ok(())
}

fn remove_pointer(value: &mut Value, pointer: &str) {
    let (parent, key) = pointer.rsplit_once('/').expect("pointer starts with /");
    if let Some(Value::Object(object)) = value.pointer_mut(parent) {
        object.remove(key);
    }
}

fn remove_key_recursively(value: &mut Value, key: &str) {
    match value {
        Value::Object(object) => {
            object.remove(key);
            object
                .values_mut()
                .for_each(|value| remove_key_recursively(value, key));
        },
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| remove_key_recursively(value, key)),
        _ => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci_helper::test_utils::{assert_error, write, Files};
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn normalize_removes_only_dynamic_fields() {
        let mut transaction = json!({
            "timestamp": 1, "version": 2, "epoch": 3, "blockHeight": 4, "sizeInfo": {},
            "info": {
                "hash": "h", "stateChangeHash": "s", "accumulatorRootHash": "r", "gasUsed": "7",
                "changes": [{
                    "writeResource": {
                        "stateKeyHash": "k", "address": "a",
                        "type": {"address": "t", "name": "keep"},
                        "data": "{\"authentication_key\":\"key\",\"nested\":{\"addr\":\"x\",\"keep\":true}}"
                    },
                    "writeTableItem": {"stateKeyHash": "k", "data": {"key": "k", "value": "v", "keep": true}}
                }]
            },
            "user": {"request": {
                "sender": "s",
                "expirationTimestampSecs": {"seconds": "10", "nanos": 1},
                "signature": {"ed25519": {"publicKey": "p", "signature": "s", "keep": true}},
                "payload": {"scriptPayload": {"code": {"bytecode": "b", "keep": true}}}
            }}
        });
        normalize_transaction(&mut transaction).unwrap();
        assert_eq!(
            transaction,
            json!({
                "info": {
                    "gasUsed": "7",
                    "changes": [{
                        "writeResource": {"type": {"name": "keep"}, "data": "{\"nested\":{\"keep\":true}}"},
                        "writeTableItem": {"data": {"keep": true}}
                    }]
                },
                "user": {"request": {
                    "expirationTimestampSecs": {"nanos": 1},
                    "signature": {"ed25519": {"keep": true}},
                    "payload": {"scriptPayload": {"code": {"keep": true}}}
                }}
            })
        );
    }

    #[test]
    fn normalize_rejects_malformed_transactions() {
        let cases = [
            (
                "no changes",
                json!({"info": {}}),
                "changes must be an array",
            ),
            (
                "embedded data",
                json!({"info": {"changes": [{"writeResource": {"data": "not-json"}}]}}),
                "not valid embedded JSON",
            ),
        ];
        for (case, mut transaction, expected) in cases {
            assert_error(normalize_transaction(&mut transaction), expected, case);
        }
    }

    #[test]
    fn compare_classifies_generated_transactions() {
        const BEFORE: &str = r#"{"timestamp": 1, "info": {"changes": []}, "value": "before"}"#;
        const BEFORE_LATER: &str =
            r#"{"timestamp": 2, "info": {"changes": []}, "value": "before"}"#;
        const AFTER: &str = r#"{"timestamp": 1, "info": {"changes": []}, "value": "after"}"#;
        let found = |diff_found, new_file_found| {
            Ok(Comparison {
                diff_found,
                new_file_found,
            })
        };
        let imported = "imported_testnet_txns/tx.json";
        let scripted = "scripted_transactions/tx.json";
        let cases: [(&str, Files, Files, Result<Comparison, &str>); 11] = [
            (
                "imported equal",
                &[(imported, BEFORE)],
                &[(imported, BEFORE)],
                found(false, false),
            ),
            (
                "imported changed",
                &[(imported, BEFORE)],
                &[(imported, AFTER)],
                found(true, false),
            ),
            (
                "imported dynamic field is not normalized",
                &[(imported, BEFORE)],
                &[(imported, BEFORE_LATER)],
                found(true, false),
            ),
            (
                "imported new",
                &[],
                &[(imported, AFTER)],
                found(false, true),
            ),
            (
                "scripted dynamic field is normalized",
                &[(scripted, BEFORE)],
                &[(scripted, BEFORE_LATER)],
                found(false, false),
            ),
            (
                "scripted changed",
                &[(scripted, BEFORE)],
                &[(scripted, AFTER)],
                found(true, false),
            ),
            (
                "scripted new",
                &[],
                &[(scripted, AFTER)],
                found(false, true),
            ),
            (
                "scripted new and malformed",
                &[],
                &[(scripted, "not json")],
                Err("is not valid JSON"),
            ),
            (
                "scripted baseline malformed",
                &[(scripted, "not json")],
                &[(scripted, AFTER)],
                Err("is not valid JSON"),
            ),
            (
                "nested scripted path",
                &[],
                &[("scripted_transactions/a/tx.json", AFTER)],
                Err("unexpected path"),
            ),
            (
                "unknown directory",
                &[],
                &[("other/tx.json", AFTER)],
                Err("unexpected path"),
            ),
        ];
        for (case, baseline_files, generated_files, expected) in cases {
            let root = TempDir::new().unwrap();
            let baseline = root.path().join("baseline");
            let generated = root.path().join("generated");
            fs::create_dir_all(&baseline).unwrap();
            fs::create_dir_all(&generated).unwrap();
            for (path, content) in baseline_files {
                write(&baseline, path, content);
            }
            for (path, content) in generated_files {
                write(&generated, path, content);
            }
            let result = compare(&baseline, &generated);
            match expected {
                Ok(expected) => assert_eq!(result.unwrap(), expected, "{case}"),
                Err(expected) => assert_error(result, expected, case),
            }
        }
    }

    #[test]
    fn dispatch_is_required_only_for_changes_without_new_files() {
        for (diff_found, new_file_found, dispatch) in [
            (false, false, false),
            (true, false, true),
            (false, true, false),
            (true, true, false),
        ] {
            let comparison = Comparison {
                diff_found,
                new_file_found,
            };
            assert_eq!(
                comparison.github_outputs(),
                format!("diff_found={diff_found}\nnew_file_found={new_file_found}\ndispatch_required={dispatch}\n")
            );
        }
    }
}
