// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Capture, replay, and interpretation support for solver timeout analysis.

use codespan::{ColumnIndex, LineIndex};
use move_model::{
    code_writer::CodeWriter,
    model::{GlobalEnv, NodeId},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::Semaphore, time::timeout_at};

const PROFILE_FREQUENCY: u64 = 1_000;
const WATCHDOG_GRACE: Duration = Duration::from_secs(2);
const DEADLINE_RESERVE: Duration = Duration::from_millis(100);
const TOP_QUANTIFIERS: usize = 5;

static STAT_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*\(?(?P<name>:[A-Za-z0-9_-]+)\s+(?P<value>[0-9]+)").unwrap());
static AUTO_QID_POSITION: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\.(?P<line>[0-9]+):(?P<column>[0-9]+)$").unwrap());

/// Raw, process-safe replay data returned to the model-owning thread.
#[derive(Debug)]
pub(crate) struct RawTimeoutAnalysis {
    pub vc_id: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub complete: bool,
    pub unavailable: Option<String>,
}

impl RawTimeoutAnalysis {
    pub(crate) fn unavailable(reason: impl Into<String>) -> Self {
        Self::unavailable_for(None, reason)
    }

    fn unavailable_for(vc_id: Option<String>, reason: impl Into<String>) -> Self {
        Self {
            vc_id,
            stdout: String::new(),
            stderr: String::new(),
            complete: false,
            unavailable: Some(reason.into()),
        }
    }
}

/// Return the seed-qualified `-proverLog` pattern for one Boogie process.
pub(crate) fn prover_log_pattern(boogie_file: &str, seed: usize) -> String {
    format!(
        "{}.seed-{}.@PROC@.smt",
        Path::new(boogie_file).with_extension("").to_string_lossy(),
        seed
    )
}

fn capture_prefix(boogie_file: &str, seed: usize) -> Option<String> {
    let base = Path::new(boogie_file).with_extension("");
    Some(format!(
        "{}.seed-{}.",
        base.file_name()?.to_string_lossy(),
        seed
    ))
}

fn is_timed_out_capture(contents: &str) -> bool {
    contents
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with(';'))
        .is_some_and(|status| {
            let status = status.to_ascii_lowercase();
            status.contains("timed out")
                || status.contains("timeout")
                || status.contains("out of resource")
        })
}

fn captured_vc_id(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix("(set-info :boogie-vc-id ")?;
        let value = rest.strip_suffix(')').unwrap_or(rest).trim();
        Some(normalize_vc_id(value))
    })
}

/// Normalize the quoting used for procedure names in Boogie text and SMT
/// `:boogie-vc-id` records.
pub(crate) fn normalize_vc_id(value: &str) -> String {
    value
        .trim()
        .trim_end_matches('.')
        .trim_matches(['\'', '"', '|'])
        .to_string()
}

fn discover_timeout_captures(boogie_file: &str, seed: usize) -> Vec<(PathBuf, String)> {
    let path = Path::new(boogie_file);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let Some(prefix) = capture_prefix(boogie_file, seed) else {
        return vec![];
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return vec![];
    };
    let mut captures = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name().is_some_and(|name| {
                let name = name.to_string_lossy();
                name.starts_with(&prefix) && name.ends_with(".smt") && !name.contains(".analysis.")
            })
        })
        .filter_map(|path| {
            let contents = fs::read_to_string(&path).ok()?;
            is_timed_out_capture(&contents).then_some((path, contents))
        })
        .collect::<Vec<_>>();
    captures.sort_by(|(left, _), (right, _)| left.cmp(right));
    captures
}

/// Whether raw Boogie output contains a solver timeout/out-of-resource result.
pub(crate) fn boogie_output_has_timeout(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    lower.contains("verification timed out")
        || lower.contains("verification out of resource")
        || lower.contains("verification of")
            && (lower.contains("timed out") || lower.contains("out of resource"))
}

/// Add replay options immediately before every solver check. Returning `None`
/// means the capture is incomplete and contains no runnable query.
fn instrument_smt(contents: &str, timeout_ms: u128) -> Option<String> {
    let mut result = String::with_capacity(contents.len().saturating_add(256));
    let mut checks = 0usize;
    for line in contents.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("(check-sat)") || trimmed.starts_with("(check-sat-assuming") {
            checks = checks.saturating_add(1);
            result.push_str("(set-option :smt.qi.profile true)\n");
            result.push_str(&format!(
                "(set-option :smt.qi.profile_freq {})\n",
                PROFILE_FREQUENCY
            ));
            result.push_str(&format!("(set-option :timeout {})\n", timeout_ms));
        }
        result.push_str(line);
    }
    (checks > 0).then_some(result)
}

fn replay_paths(capture: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        capture.with_extension("analysis.smt"),
        capture.with_extension("analysis.stdout"),
        capture.with_extension("analysis.stderr"),
    )
}

fn read_lossy(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

struct ReplayArtifactGuard {
    paths: [PathBuf; 3],
    keep: bool,
}

impl Drop for ReplayArtifactGuard {
    fn drop(&mut self) {
        if !self.keep {
            for path in &self.paths {
                fs::remove_file(path).unwrap_or_default();
            }
        }
    }
}

async fn replay_capture(
    z3_exe: &str,
    capture: PathBuf,
    contents: String,
    root_timeout_secs: usize,
    process_deadline: Option<Instant>,
    keep_artifacts: bool,
    process_sem: Arc<Semaphore>,
) -> RawTimeoutAnalysis {
    let vc_id = captured_vc_id(&contents);
    let nominal = Duration::from_secs(root_timeout_secs as u64);
    if nominal.is_zero() {
        return RawTimeoutAnalysis::unavailable_for(
            vc_id,
            "the verification root has no soft timeout",
        );
    }
    let _permit = match process_sem.acquire().await {
        Ok(permit) => permit,
        Err(_) => {
            return RawTimeoutAnalysis::unavailable_for(
                vc_id,
                "the prover process queue was closed",
            );
        },
    };
    let now = Instant::now();
    let budget = if let Some(deadline) = process_deadline {
        let Some(remaining) = deadline.checked_duration_since(now) else {
            return RawTimeoutAnalysis::unavailable_for(
                vc_id,
                "the request deadline was exhausted",
            );
        };
        if remaining <= DEADLINE_RESERVE {
            return RawTimeoutAnalysis::unavailable_for(
                vc_id,
                "the request deadline was exhausted",
            );
        }
        nominal.min(remaining.saturating_sub(DEADLINE_RESERVE))
    } else {
        nominal
    };
    let timeout_ms = budget.as_millis().max(1);
    let Some(instrumented) = instrument_smt(&contents, timeout_ms) else {
        return RawTimeoutAnalysis::unavailable_for(vc_id, "the captured SMT query is incomplete");
    };
    let (replay_path, stdout_path, stderr_path) = replay_paths(&capture);
    let _artifact_guard = ReplayArtifactGuard {
        paths: [
            replay_path.clone(),
            stdout_path.clone(),
            stderr_path.clone(),
        ],
        keep: keep_artifacts,
    };
    if let Err(err) = fs::write(&replay_path, instrumented) {
        log::debug!("cannot write timeout-analysis replay: {}", err);
        return RawTimeoutAnalysis::unavailable_for(vc_id, "the replay query could not be written");
    }
    let stdout_file = match fs::File::create(&stdout_path) {
        Ok(file) => file,
        Err(err) => {
            log::debug!("cannot create timeout-analysis stdout: {}", err);
            return RawTimeoutAnalysis::unavailable_for(
                vc_id,
                "the replay output could not be created",
            );
        },
    };
    let stderr_file = match fs::File::create(&stderr_path) {
        Ok(file) => file,
        Err(err) => {
            log::debug!("cannot create timeout-analysis stderr: {}", err);
            return RawTimeoutAnalysis::unavailable_for(
                vc_id,
                "the replay output could not be created",
            );
        },
    };
    let child = Command::new(z3_exe)
        .arg("-st")
        .arg(&replay_path)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .kill_on_drop(true)
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(err) => {
            log::debug!("cannot launch timeout-analysis Z3 replay: {}", err);
            return RawTimeoutAnalysis::unavailable_for(
                vc_id,
                "the Z3 replay could not be launched",
            );
        },
    };

    let watchdog = now
        .checked_add(budget.saturating_add(WATCHDOG_GRACE))
        .into_iter()
        .chain(process_deadline)
        .min()
        .unwrap_or(now);
    let (complete, status_ok) =
        match timeout_at(tokio::time::Instant::from_std(watchdog), child.wait()).await {
            Ok(Ok(status)) => (true, status.success()),
            Ok(Err(err)) => {
                log::debug!("cannot wait for timeout-analysis Z3 replay: {}", err);
                (false, false)
            },
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                (false, true)
            },
        };
    let stdout = read_lossy(&stdout_path);
    let stderr = read_lossy(&stderr_path);
    if !status_ok && stdout.is_empty() && stderr.is_empty() {
        RawTimeoutAnalysis::unavailable_for(
            vc_id,
            "the Z3 replay failed without producing evidence",
        )
    } else {
        RawTimeoutAnalysis {
            vc_id,
            stdout,
            stderr,
            complete: complete && status_ok,
            unavailable: None,
        }
    }
}

/// Analyze the selected seed's timed-out captures. This runs after the seed
/// race and acquires the invocation-wide process semaphore for every replay.
pub(crate) async fn analyze_selected_seed(
    z3_exe: &str,
    boogie_file: &str,
    seed: usize,
    root_timeout_secs: usize,
    process_deadline: Option<Instant>,
    keep_artifacts: bool,
    process_sem: Arc<Semaphore>,
) -> Vec<RawTimeoutAnalysis> {
    let captures = discover_timeout_captures(boogie_file, seed);
    if captures.is_empty() {
        return vec![RawTimeoutAnalysis::unavailable(
            "no timed-out SMT capture was found",
        )];
    }
    let mut results = Vec::with_capacity(captures.len());
    for (path, contents) in captures {
        results.push(
            replay_capture(
                z3_exe,
                path,
                contents,
                root_timeout_secs,
                process_deadline,
                keep_artifacts,
                process_sem.clone(),
            )
            .await,
        );
    }
    results
}

fn vc_matches(procedure: Option<&str>, vc_id: Option<&str>) -> bool {
    match (procedure, vc_id) {
        (_, None) | (None, _) => true,
        (Some(procedure), Some(vc_id)) => {
            vc_id == procedure
                || vc_id
                    .strip_prefix(procedure)
                    .is_some_and(|suffix| suffix.starts_with("_split"))
        },
    }
}

fn parse_profile_counts(output: &str) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::<String, u64>::new();
    for line in output.lines() {
        let Some(rest) = line.strip_prefix("[quantifier_instances]") else {
            continue;
        };
        let mut fields = rest.split(" : ");
        let Some(qid) = fields.next().map(str::trim).filter(|qid| !qid.is_empty()) else {
            continue;
        };
        let Some(count) = fields
            .next()
            .and_then(|count| count.trim().parse::<u64>().ok())
        else {
            continue;
        };
        counts
            .entry(qid.to_string())
            .and_modify(|current| *current = (*current).max(count))
            .or_insert(count);
    }
    counts
}

fn parse_stats(output: &str) -> BTreeMap<String, u64> {
    STAT_LINE
        .captures_iter(output)
        .filter_map(|capture| {
            let name = capture.name("name")?.as_str().trim_start_matches(':');
            let value = capture.name("value")?.as_str().parse::<u64>().ok()?;
            Some((name.to_string(), value))
        })
        .collect()
}

fn is_nonlinear_stat(name: &str) -> bool {
    name.starts_with("arith-nla-")
        || name.starts_with("arith-grobner-")
        || name.starts_with("nlsat-")
}

fn resolve_spec_fun_qid(fields: &[&str], env: &GlobalEnv) -> Option<String> {
    if fields.len() != 5
        || fields[0] != "move"
        || fields[1] != "spec_fun"
        || fields[4].parse::<usize>().is_err()
    {
        return None;
    }
    let node = NodeId::new(fields[2].parse::<usize>().ok()?);
    let source_name = fields[3];
    let loc = env.get_node_loc(node);
    Some(
        if loc != env.unknown_loc() && loc != env.internal_loc() {
            format!(
                "definition of spec function {} {}",
                source_name,
                loc.display_file_name_and_line(env)
            )
        } else {
            format!("definition of spec function {}", source_name)
        },
    )
}

fn humanize_legacy_definition_qid(base: &str) -> String {
    let symbol = base.strip_suffix(".def").unwrap_or(base);
    let symbol = symbol.strip_prefix('$').unwrap_or(symbol);
    let symbol = symbol
        .split_once('_')
        .filter(|(prefix, _)| !prefix.is_empty() && prefix.chars().all(|ch| ch.is_ascii_hexdigit()))
        .map_or(symbol, |(_, rest)| rest);
    format!("generated definition for {}", symbol)
}

fn resolve_qid(
    qid: &str,
    env: &GlobalEnv,
    writer: &CodeWriter,
    boogie_text: Option<&str>,
) -> String {
    let fields = qid.split('.').collect::<Vec<_>>();
    if let Some(resolved) = resolve_spec_fun_qid(&fields, env) {
        return resolved;
    }
    if fields.len() == 4 && fields[0] == "move" && matches!(fields[1], "forall" | "exists") {
        if let Ok(node) = fields[2].parse::<usize>() {
            let loc = env.get_node_loc(NodeId::new(node));
            if loc != env.unknown_loc() && loc != env.internal_loc() {
                return format!("{} {}", fields[1], loc.display_file_name_and_line(env));
            }
        }
    }
    if let Some((base, ordinal)) = qid.rsplit_once('.') {
        if ordinal.parse::<usize>().is_ok() && base.ends_with(".def") {
            return humanize_legacy_definition_qid(base);
        }
    }
    let Some(capture) = AUTO_QID_POSITION.captures(qid) else {
        return qid.to_string();
    };
    let Some(line) = capture
        .name("line")
        .and_then(|value| value.as_str().parse::<u32>().ok())
    else {
        return qid.to_string();
    };
    let Some(column) = capture
        .name("column")
        .and_then(|value| value.as_str().parse::<u32>().ok())
    else {
        return qid.to_string();
    };
    let line_index = LineIndex(line.saturating_sub(1));
    let column_index = ColumnIndex(column);
    if let Some(index) = writer.get_output_byte_index(line_index, column_index) {
        if let Some(loc) = writer.get_source_location(index) {
            if loc != env.unknown_loc() && loc != env.internal_loc() {
                return format!("quantifier {}", loc.display_file_name_and_line(env));
            }
        }
    }
    let Some(text) = boogie_text else {
        return qid.to_string();
    };
    let lines = text.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return qid.to_string();
    }
    let at = line.saturating_sub(1) as usize;
    for index in (0..=at.min(lines.len().saturating_sub(1))).rev() {
        let declaration = lines[index].trim();
        if ["axiom", "function", "procedure"]
            .iter()
            .any(|prefix| declaration.starts_with(prefix))
        {
            let declaration = declaration.chars().take(120).collect::<String>();
            let comment = lines[..index]
                .iter()
                .rev()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim())
                .filter(|line| line.starts_with("//"));
            return comment.map_or(declaration.clone(), |comment| {
                format!(
                    "{} — {}",
                    comment.trim_start_matches('/').trim(),
                    declaration
                )
            });
        }
    }
    qid.to_string()
}

fn format_count(value: u64) -> String {
    let digits = value.to_string();
    let mut result = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// Render evidence for one timeout diagnostic after source-map resolution.
pub(crate) fn render_notes(
    raw: &[RawTimeoutAnalysis],
    procedure: Option<&str>,
    env: &GlobalEnv,
    writer: &CodeWriter,
    boogie_file: &str,
    stable_test_output: bool,
) -> Vec<String> {
    if stable_test_output {
        return vec![
            "timeout analysis requested; runtime evidence redacted for stable output".to_string(),
        ];
    }
    let relevant = raw
        .iter()
        .filter(|result| vc_matches(procedure, result.vc_id.as_deref()))
        .collect::<Vec<_>>();
    if relevant.is_empty() {
        return vec!["timeout analysis unavailable: no matching SMT capture was found".to_string()];
    }
    let boogie_text = fs::read_to_string(boogie_file).ok();
    let mut resolved_counts = BTreeMap::<String, u64>::new();
    let mut quant_instantiations = 0u64;
    let mut saw_quant_evidence = false;
    let mut nonlinear = BTreeMap::<String, u64>::new();
    let mut incomplete = false;
    let mut unavailable = vec![];
    for result in relevant {
        if let Some(reason) = &result.unavailable {
            unavailable.push(reason.clone());
            incomplete = true;
            continue;
        }
        let output = format!("{}\n{}", result.stdout, result.stderr);
        let stats = parse_stats(&output);
        let profile = parse_profile_counts(&output);
        let local_quant_instantiations = stats
            .get("quant-instantiations")
            .copied()
            .unwrap_or_else(|| profile.values().copied().sum());
        saw_quant_evidence |= stats.contains_key("quant-instantiations") || !profile.is_empty();
        quant_instantiations = quant_instantiations.saturating_add(local_quant_instantiations);
        for (name, value) in stats
            .into_iter()
            .filter(|(name, _)| is_nonlinear_stat(name))
        {
            let total = nonlinear.entry(name).or_default();
            *total = total.saturating_add(value);
        }
        for (qid, count) in profile {
            let display = resolve_qid(&qid, env, writer, boogie_text.as_deref());
            let total = resolved_counts.entry(display).or_default();
            *total = total.saturating_add(count);
        }
        incomplete |= !result.complete || !stats_contains_aggregate(&output);
    }
    if !saw_quant_evidence && nonlinear.is_empty() {
        let reason = unavailable
            .first()
            .map(String::as_str)
            .unwrap_or("the replay produced no recognized statistics");
        return vec![format!("timeout analysis unavailable: {}", reason)];
    }

    let quantifier_activity = quant_instantiations > 0 || !resolved_counts.is_empty();
    let nonlinear_activity = nonlinear.values().any(|value| *value > 0);
    let signature = match (quantifier_activity, nonlinear_activity) {
        (true, true) => "mixed quantifier and nonlinear arithmetic activity",
        (true, false) => "quantifier activity",
        (false, true) => "nonlinear arithmetic activity",
        (false, false) => "no clear activity signature",
    };
    let mut evidence = vec![];
    if quantifier_activity {
        evidence.push(format!(
            "{} observed quantifier instantiations",
            format_count(quant_instantiations)
        ));
    }
    if nonlinear_activity {
        let mut counters = nonlinear.into_iter().collect::<Vec<_>>();
        counters.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        evidence.push(
            counters
                .into_iter()
                .take(3)
                .map(|(name, value)| format!("{}={}", name, format_count(value)))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    let detail = if evidence.is_empty() {
        String::new()
    } else {
        format!(" ({})", evidence.join("; "))
    };
    let mut notes = vec![format!("timeout analysis replay: {}{}", signature, detail)];
    if !resolved_counts.is_empty() {
        let mut top = resolved_counts.into_iter().collect::<Vec<_>>();
        top.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        notes.push(format!(
            "top quantifier activity:\n{}",
            top.into_iter()
                .take(TOP_QUANTIFIERS)
                .map(|(name, count)| {
                    format!(
                        "  {} — {}+ observed instantiations",
                        name,
                        format_count(count)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if incomplete {
        notes.push(
            "timeout analysis incomplete; only partial replay evidence was available".to_string(),
        );
    }
    notes
}

fn stats_contains_aggregate(output: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim_start().starts_with("(:"))
}

/// Remove timeout-analysis captures and replay products after their evidence
/// has been consumed. Kept-artifact runs deliberately retain all of them.
pub(crate) fn cleanup_artifacts(boogie_file: &str) {
    let path = Path::new(boogie_file);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let Some(base) = path
        .with_extension("")
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
    else {
        return;
    };
    let prefix = format!("{}.seed-", base);
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        let matches = path.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name.starts_with(&prefix)
                && (name.ends_with(".smt")
                    || name.ends_with(".analysis.stdout")
                    || name.ends_with(".analysis.stderr"))
        });
        if matches {
            fs::remove_file(path).unwrap_or_default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_command_line_common::files::FileHash;
    use move_model::{
        model::{GlobalEnv, Loc},
        ty::BOOL_TYPE,
    };
    use std::{
        rc::Rc,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "move-prover-timeout-analysis-{}-{}-{}",
            label,
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn capture_pattern_is_seed_qualified() {
        assert_eq!(
            prover_log_pattern("/tmp/output.vc_0001.bpl", 7),
            "/tmp/output.vc_0001.seed-7.@PROC@.smt"
        );
    }

    #[test]
    fn instrumentation_precedes_every_check() {
        let input = "(reset)\n(check-sat)\n(check-sat-assuming (a))\n";
        let output = instrument_smt(input, 1234).unwrap();
        assert_eq!(output.matches(":smt.qi.profile true").count(), 2);
        assert_eq!(output.matches(":smt.qi.profile_freq 1000").count(), 2);
        assert_eq!(output.matches(":timeout 1234").count(), 2);
        assert!(output.find(":timeout 1234").unwrap() < output.find("(check-sat)").unwrap());
    }

    #[test]
    fn profile_parser_keeps_latest_cumulative_count() {
        let output = "\
[quantifier_instances] q.one : 1000 : 0 : 0 : 0 : 1\n\
[quantifier_instances] q.one : 2000 : 0 : 0 : 0 : 1\n\
[quantifier_instances] q.two : 7 : 0 : 0 : 0 : 1\n";
        assert_eq!(
            parse_profile_counts(output),
            BTreeMap::from([("q.one".to_string(), 2000), ("q.two".to_string(), 7)])
        );
    }

    #[test]
    fn only_explicit_nonlinear_stats_are_selected() {
        assert!(!is_nonlinear_stat("arith-make-feasible"));
        assert!(is_nonlinear_stat("arith-nla-lemmas"));
        assert!(is_nonlinear_stat("arith-grobner-calls"));
        assert!(is_nonlinear_stat("nlsat-conflicts"));
    }

    #[test]
    fn parses_integer_statistics() {
        let output = "(:arith-make-feasible 2\n :quant-instantiations 48231\n :total-time 40.01)";
        let stats = parse_stats(output);
        assert_eq!(stats.get("quant-instantiations"), Some(&48231));
        assert_eq!(stats.get("arith-make-feasible"), Some(&2));
    }

    #[test]
    fn capture_status_comes_from_terminal_footer() {
        let common = "(set-option :timeout 1000)\n(check-sat)\n";
        assert!(!is_timed_out_capture(&format!("{}; Invalid\n", common)));
        assert!(is_timed_out_capture(&format!("{}; Timed out\n", common)));
        assert!(is_timed_out_capture(&format!(
            "{}; Out of resource\n",
            common
        )));
    }

    #[test]
    fn split_captures_match_their_parent_procedure() {
        assert!(vc_matches(Some("p"), Some("p")));
        assert!(vc_matches(Some("p"), Some("p_split0")));
        assert!(!vc_matches(Some("p"), Some("other_split0")));
        assert_eq!(normalize_vc_id("'$1_p$verify'."), "$1_p$verify");
    }

    #[test]
    fn capture_discovery_is_seed_scoped_and_split_aware() {
        let dir = test_dir("discovery");
        let boogie = dir.join("root.bpl");
        fs::write(&boogie, "").unwrap();
        for (name, footer) in [
            ("root.seed-7.p.smt", "Timed out"),
            ("root.seed-7.p_split0.smt", "Out of resource"),
            ("root.seed-7.ok.smt", "Valid"),
            ("root.seed-8.other.smt", "Timed out"),
            ("root.seed-7.p.analysis.smt", "Timed out"),
        ] {
            fs::write(
                dir.join(name),
                format!("(set-info :boogie-vc-id p)\n(check-sat)\n; {}\n", footer),
            )
            .unwrap();
        }
        let captures = discover_timeout_captures(boogie.to_str().unwrap(), 7);
        assert_eq!(captures.len(), 2);
        assert!(captures[0]
            .0
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("p.smt"));
        assert!(captures[1]
            .0
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("p_split0.smt"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fallback_qid_resolution_names_an_enclosing_declaration() {
        let env = GlobalEnv::new();
        let writer = CodeWriter::new(env.unknown_loc());
        let boogie = "// sequence theory\naxiom (forall x: int :: true);\n";
        let resolved = resolve_qid("probe.2:0", &env, &writer, Some(boogie));
        assert!(resolved.contains("sequence theory"));
        assert!(resolved.contains("axiom"));
    }

    #[test]
    fn stable_rendering_redacts_runtime_evidence() {
        let env = GlobalEnv::new();
        let writer = CodeWriter::new(env.unknown_loc());
        assert_eq!(
            render_notes(&[], None, &env, &writer, "missing.bpl", true),
            vec!["timeout analysis requested; runtime evidence redacted for stable output"]
        );
    }

    #[test]
    fn recorded_evidence_renders_without_causal_claims() {
        let env = GlobalEnv::new();
        let writer = CodeWriter::new(env.unknown_loc());
        let raw = RawTimeoutAnalysis {
            vc_id: Some("p".to_string()),
            stdout: "(:quant-instantiations 1234\n :arith-nla-lemmas 9)".to_string(),
            stderr: "[quantifier_instances] move.forall.999.0 : 1200 : 0 : 0 : 0 : 1".to_string(),
            complete: true,
            unavailable: None,
        };
        let notes = render_notes(&[raw], Some("p"), &env, &writer, "missing.bpl", false);
        assert!(notes[0].contains("mixed quantifier and nonlinear arithmetic activity"));
        assert!(notes[0].contains("1,234 observed quantifier instantiations"));
        assert!(!notes.join("\n").contains("dominated"));
    }

    fn review_diagnostic(title: &str, notes: Vec<String>) -> String {
        let mut result = format!(
            "=== {} ===\nerror: verification out of resources/timeout (timeout set to 1s)\n",
            title
        );
        for note in notes {
            let mut lines = note.lines();
            if let Some(line) = lines.next() {
                result.push_str("  = ");
                result.push_str(line);
                result.push('\n');
            }
            for line in lines {
                result.push_str("    ");
                result.push_str(line);
                result.push('\n');
            }
        }
        result
    }

    #[test]
    fn human_readable_diagnostic_baseline() {
        let source = "module 0x1::staking_contract {\n\
    spec fun spec_fold(n: u64): u64 { n }\n\
    spec fun check(xs: vector<u64>) {\n\
        ensures forall i: u64 where i < len(xs): xs[i] >= 0;\n\
    }\n\
}\n";
        let mut env = GlobalEnv::new();
        let file_id = env.add_source(
            FileHash::new(source),
            Rc::new(BTreeMap::new()),
            "sources/staking_contract.move",
            source,
            true,
            true,
        );
        let definition_start = source.find("spec_fold").unwrap() as u32;
        let definition_node = env.new_node(
            Loc::new(
                file_id,
                codespan::Span::new(definition_start, definition_start + 9),
            ),
            BOOL_TYPE,
        );
        assert_eq!(definition_node, NodeId::new(0));
        let start = source.find("forall").unwrap() as u32;
        let quantifier_node = env.new_node(
            Loc::new(file_id, codespan::Span::new(start, start + 6)),
            BOOL_TYPE,
        );
        assert_eq!(quantifier_node, NodeId::new(1));
        let writer = CodeWriter::new(env.unknown_loc());

        let mixed = RawTimeoutAnalysis {
            vc_id: Some("staking_contract_check".to_string()),
            stdout: "(:quant-instantiations 48231\n :arith-nla-propagate-bounds 3102\n :arith-nla-lemmas 17)".to_string(),
            stderr: "[quantifier_instances] move.spec_fun.0.0x1::staking_contract::spec_fold.0 : 31204 : 0 : 0 : 0 : 1\n\
[quantifier_instances] move.forall.1.0 : 9876 : 0 : 0 : 0 : 1".to_string(),
            complete: true,
            unavailable: None,
        };
        let partial = RawTimeoutAnalysis {
            vc_id: Some("staking_contract_check".to_string()),
            stdout: String::new(),
            stderr: "[quantifier_instances] move.forall.1.0 : 20800 : 0 : 0 : 0 : 1".to_string(),
            complete: false,
            unavailable: None,
        };
        let unavailable = RawTimeoutAnalysis::unavailable_for(
            Some("staking_contract_check".to_string()),
            "the replay deadline expired before this capture could run",
        );

        let mut actual = String::new();
        for (index, (title, raw, stable)) in [
            ("mixed replay evidence", vec![mixed], false),
            ("partial replay evidence", vec![partial], false),
            ("replay unavailable", vec![unavailable], false),
            ("stable test output", vec![], true),
        ]
        .into_iter()
        .enumerate()
        {
            if index > 0 {
                actual.push('\n');
            }
            actual.push_str(&review_diagnostic(
                title,
                render_notes(
                    &raw,
                    Some("staking_contract_check"),
                    &env,
                    &writer,
                    "missing.bpl",
                    stable,
                ),
            ));
        }

        assert_eq!(
            actual,
            include_str!("../tests/timeout_analysis/readability.exp")
        );
        assert!(!actual.contains('$'));
    }

    #[test]
    fn legacy_definition_qids_are_humanized() {
        assert_eq!(
            humanize_legacy_definition_qid("$1_staking_contract_spec_fold.def"),
            "generated definition for staking_contract_spec_fold"
        );
        assert_eq!(
            humanize_legacy_definition_qid("$spec_fun.def"),
            "generated definition for spec_fun"
        );
    }

    #[test]
    fn replay_failures_preserve_the_base_timeout_evidence_path() {
        let dir = test_dir("replay-failure");
        let capture = dir.join("root.seed-1.p.smt");
        let contents = "(set-info :boogie-vc-id p)\n(check-sat)\n; Timed out\n".to_string();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let failed = runtime.block_on(replay_capture(
            "/definitely/missing/z3",
            capture.clone(),
            contents.clone(),
            1,
            None,
            false,
            Arc::new(Semaphore::new(1)),
        ));
        assert_eq!(failed.vc_id.as_deref(), Some("p"));
        assert!(failed.unavailable.is_some());
        assert!(!capture.with_extension("analysis.smt").exists());

        let exhausted = runtime.block_on(replay_capture(
            "/definitely/missing/z3",
            capture,
            contents,
            1,
            Some(Instant::now() - Duration::from_millis(1)),
            false,
            Arc::new(Semaphore::new(1)),
        ));
        assert!(exhausted
            .unavailable
            .as_deref()
            .unwrap()
            .contains("deadline"));
        fs::remove_dir_all(dir).unwrap();
    }
}
