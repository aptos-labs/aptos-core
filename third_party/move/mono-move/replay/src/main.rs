// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! CLI for the MonoMove-vs-AptosVM replay comparison.
//!
//! - `capture` fetches transactions from chain into a corpus (each transaction plus its read-set
//!   with the full module dependency closure, and its on-chain status).
//! - `targets` picks versions to capture: recent calls of every entry function used in a window.
//! - `compare` replays a corpus on both the legacy AptosVM (V1) and the MonoMove-backed
//!   transaction executor (V2) and reports, per record, whether the outputs match.
//! - `replay` prints one record's full output on V1, V2 or both.
//! - `aggregate` merges `compare` results into one summary.
//! - `survey` and `import` read dumps written by `aptos-e2e-comparison-testing`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use mono_move_replay::{
    aggregate, capture, comparison,
    corpus::{self, Corpus, FrameworkSource},
    import::{self, ImportConfig, RestChainSource},
    isolated::{self, V2Limits},
    legacy::LegacyDump,
    replay, targets,
};
use mono_move_replay_common::cli::{Network, VMSelection};
use std::{
    collections::{BTreeSet, HashSet},
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    about = "Compare the MonoMove transaction executor (V2) against the legacy AptosVM (V1) on \
             real chain transactions."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Capture transactions from chain into a self-contained dump.
    Capture(CaptureArgs),
    /// Decode a dump written by `aptos-e2e-comparison-testing` and summarize what it holds.
    Survey(SurveyArgs),
    /// Convert a dump written by `aptos-e2e-comparison-testing` into a corpus.
    Import(ImportArgs),
    /// Compare MonoMove against AptosVM on the records of a corpus.
    Compare(CompareArgs),
    /// Replay records of a corpus on one VM or both and print each full output: status, gas,
    /// write set and events.
    Replay(ReplayArgs),
    /// Pick versions to capture: recent calls of every entry function used in a version window.
    Targets(TargetsArgs),
    /// Merge the results of `compare` runs into one summary; fails if any record mismatched.
    Aggregate(AggregateArgs),
    /// Internal: runs one MonoMove replay as a child process; see `isolated`.
    #[clap(name = isolated::WORKER_SUBCOMMAND, hide = true)]
    V2Worker,
}

/// The limits each MonoMove replay runs under, in a child process: a replay that exceeds one, or
/// whose process dies, is reported as a crash instead of ending the run.
#[derive(clap::Args)]
struct LimitArgs {
    #[clap(
        long,
        default_value_t = 4096,
        value_parser = clap::value_parser!(u64).range(1..),
        help = "Memory limit of each MonoMove replay, in MB; replays run at once only while their \
                limits fit in three quarters of the host's (or container's) memory"
    )]
    v2_memory_limit_mb: u64,
    #[clap(
        long,
        default_value_t = 120,
        value_parser = clap::value_parser!(u64).range(1..),
        help = "Time limit of each MonoMove replay, in seconds"
    )]
    v2_timeout_secs: u64,
    #[clap(
        long,
        value_parser = clap::value_parser!(u64).range(1..),
        help = "Memory all MonoMove replays of this process may use at once, in MB (default: three \
                quarters of the host's or container's memory). It limits how many replays run at \
                once, not what each may use. The budget is per process: give each of several \
                processes on one host a share"
    )]
    v2_memory_budget_mb: Option<u64>,
}

impl LimitArgs {
    /// The limits, fitted to the memory budget; the same for every command. Also sets the budget,
    /// and identifies the worker binary before the command does anything else.
    fn limits(&self) -> Result<V2Limits> {
        let limits = V2Limits {
            memory_mb: self.v2_memory_limit_mb,
            timeout: std::time::Duration::from_secs(self.v2_timeout_secs),
            // Starting (spawning, receiving and decoding the request) is held to MonoMove's bound,
            // but never below a minute, so that a short MonoMove limit does not fail a slow start.
            startup_timeout: std::time::Duration::from_secs(self.v2_timeout_secs.max(60)),
        }
        .fit_to_host();
        if let Some(mb) = self.v2_memory_budget_mb {
            // A budget below one replay's reservation, its (fitted) limit plus its start-up
            // allowance, would still let a replay run alone above it, so it could not bound the
            // host: the limit must be lowered, knowingly.
            let least = limits.memory_mb.saturating_add(isolated::MIN_STARTUP_MB);
            anyhow::ensure!(
                mb >= least,
                "--v2-memory-budget-mb ({} MB) is below one replay's reservation, at least its \
                 memory limit plus {} MB ({} MB); lower --v2-memory-limit-mb too, knowing that it \
                 changes what MonoMove may use",
                mb,
                isolated::MIN_STARTUP_MB,
                least
            );
            isolated::set_memory_budget_mb(mb)?;
        }
        isolated::pin_worker()?;
        Ok(limits)
    }
}

#[derive(Parser)]
struct CaptureArgs {
    #[clap(flatten)]
    limits: LimitArgs,
    #[clap(
        long,
        default_value = "mainnet",
        help = "Network to capture from: mainnet, testnet, devnet, or a custom REST endpoint URL"
    )]
    network: String,
    #[clap(long, help = "Optional API key to raise the request-rate quota")]
    api_key: Option<String>,
    #[clap(long, help = "First transaction version to capture (inclusive)")]
    begin_version: Option<u64>,
    #[clap(long, help = "Last transaction version to capture (inclusive)")]
    end_version: Option<u64>,
    #[clap(
        long,
        help = "File of versions to capture, one per line (instead of a version range)"
    )]
    versions_file: Option<PathBuf>,
    #[clap(
        long,
        help = "Directory to write the corpus to; must not hold a corpus already"
    )]
    out: PathBuf,
    #[clap(long, help = "Id of the new corpus, e.g. mainnet-2026-10-05")]
    corpus_id: String,
    #[clap(long, default_value_t = 500, help = "Records per shard")]
    shard_size: usize,
    #[clap(
        long,
        default_value_t = 32,
        help = "Transactions captured at once. Higher is faster, until the node rate-limits: without \
                an API key, mainnet's public endpoint returned 429s at 64"
    )]
    concurrency: usize,
    #[clap(
        long,
        help = "Also replay both VMs on the captured state and fetch every key either reads that V1 \
                did not on chain (state completion). Off by default: V1's reads have sufficed on \
                mainnet so far, and a record MonoMove reads beyond compares as `incomplete`, so it \
                can be recaptured with this flag."
    )]
    complete: bool,
}

#[derive(Parser)]
struct SurveyArgs {
    #[clap(long, help = "Unpacked legacy dump directory")]
    legacy_dump: PathBuf,
    #[clap(
        long,
        default_value_t = 500,
        help = "Records per shard, for the value pooling estimate"
    )]
    shard_size: usize,
    #[clap(long, help = "Print the survey as JSON")]
    json: bool,
}

/// The framework to pair legacy records with.
#[derive(Clone, Copy, ValueEnum)]
enum FrameworkArg {
    /// The on-chain framework at each record's version (needs chain access).
    Onchain,
    /// The framework this binary was built with.
    Head,
}

#[derive(Parser)]
struct AggregateArgs {
    #[clap(
        long,
        num_args = 1..,
        required = true,
        help = "Result files written by `compare --out`, or directories of `*.jsonl` files"
    )]
    results: Vec<PathBuf>,
    #[clap(long, help = "File to write the summary to, as JSON")]
    summary_out: Option<PathBuf>,
    #[clap(long, help = "File to write a Markdown report to")]
    markdown: Option<PathBuf>,
}

#[derive(Parser)]
struct TargetsArgs {
    #[clap(
        long,
        help = "First version of the window whose entry functions to target"
    )]
    from_version: u64,
    #[clap(
        long,
        help = "Last version of the window; calls are picked at or before it"
    )]
    to_version: u64,
    #[clap(
        long,
        default_value_t = 3,
        help = "Most recent calls to pick per entry function"
    )]
    per_function: usize,
    #[clap(
        long,
        default_value = "https://api.mainnet.aptoslabs.com/v1/graphql",
        help = "Indexer GraphQL endpoint"
    )]
    indexer_url: String,
    #[clap(
        long,
        help = "Optional API key to raise the indexer's request-rate quota"
    )]
    api_key: Option<String>,
    #[clap(
        long,
        help = "File to write the versions to, one per line, for capture"
    )]
    out: PathBuf,
}

#[derive(Parser)]
struct ReplayArgs {
    #[clap(flatten)]
    limits: LimitArgs,
    #[clap(long, help = "Corpus directory")]
    corpus: PathBuf,
    #[clap(
        long,
        num_args = 1,
        value_delimiter = ',',
        required = true,
        help = "Versions to replay"
    )]
    versions: Vec<u64>,
    #[clap(long, value_enum, default_value = "both", help = "Which VM(s) to run")]
    vm: VMSelection,
    #[clap(
        long,
        help = "File to write the outputs to (default: stdout); each VM's output is flushed \
                before the next VM runs"
    )]
    out: Option<PathBuf>,
}

#[derive(Parser)]
struct CompareArgs {
    #[clap(flatten)]
    limits: LimitArgs,
    #[clap(long, help = "Corpus directory")]
    corpus: PathBuf,
    #[clap(
        long,
        num_args = 1,
        value_delimiter = ',',
        help = "Shards to compare (default: all)"
    )]
    shards: Vec<usize>,
    #[clap(
        long,
        num_args = 1,
        value_delimiter = ',',
        help = "Compare only these versions"
    )]
    versions: Vec<u64>,
    #[clap(
        long,
        help = "File to write one JSON line per record to (default: stdout)"
    )]
    out: Option<PathBuf>,
}

#[derive(Parser)]
struct ImportArgs {
    #[clap(flatten)]
    limits: LimitArgs,
    #[clap(long, help = "Unpacked legacy dump directory")]
    legacy_dump: PathBuf,
    #[clap(
        long,
        help = "Directory to write the corpus to; must not hold a corpus already"
    )]
    out: PathBuf,
    #[clap(long, help = "Id of the new corpus, e.g. legacy-mainnet-1-2m")]
    corpus_id: String,
    #[clap(
        long,
        default_value = "mainnet",
        help = "Network the dump was taken from: mainnet, testnet, devnet, or a custom REST \
                endpoint URL"
    )]
    network: String,
    #[clap(long, help = "Optional API key to raise the request-rate quota")]
    api_key: Option<String>,
    #[clap(long, value_enum, default_value = "onchain")]
    framework: FrameworkArg,
    #[clap(
        long,
        help = "Do not access the chain: implies --framework head, leaves the on-chain status \
                empty, and skips records whose module closure is incomplete. Without state \
                completion the records lack the chain's feature flags and compare as incomplete"
    )]
    offline: bool,
    #[clap(
        long,
        help = "Directory caching chain reads across runs, which makes an interrupted import \
                resumable"
    )]
    cache_dir: Option<PathBuf>,
    #[clap(
        long,
        help = "Do not fetch the keys a replay reads beyond the dump; such records then compare \
                as incomplete"
    )]
    skip_completion: bool,
    #[clap(long, default_value_t = 500, help = "Records per shard")]
    shard_size: usize,
    #[clap(
        long,
        help = "Import at most this many records, from the lowest version"
    )]
    limit: Option<usize>,
    #[clap(
        long,
        default_value_t = 8,
        help = "Concurrent requests when fetching committed transactions"
    )]
    concurrency: usize,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Capture(args) => capture(args),
        Command::Survey(args) => survey(args),
        Command::Import(args) => import(args),
        Command::Compare(args) => compare(args),
        Command::Replay(args) => replay(args),
        Command::Targets(args) => targets(args),
        Command::Aggregate(args) => aggregate(args),
        Command::V2Worker => isolated::worker_main(),
    }
}

fn survey(args: SurveyArgs) -> Result<()> {
    let survey = LegacyDump::open(&args.legacy_dump)?.survey(args.shard_size)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&survey)?);
        return Ok(());
    }
    let mb = |bytes: u64| bytes as f64 / (1 << 20) as f64;
    println!("dump:        {:?}", args.legacy_dump);
    println!("era:         {:?}", survey.era);
    println!(
        "versions:    {:?} ..= {:?}",
        survey.first_version, survey.last_version
    );
    println!(
        "records:     {} indexed, {} in version_index.txt, {} state files \
         ({} indexed without state, {} state without index)",
        survey.indexed,
        survey.index_file_lines,
        survey.state_files,
        survey.indexed_without_state,
        survey.state_without_index
    );
    println!(
        "decoded:     {} ok, {} failed",
        survey.decoded, survey.decode_errors
    );
    if let Some(err) = &survey.first_error {
        println!("first error: {}", err);
    }
    println!("txn kinds:   {:?}", survey.txn_kinds);
    let mut packages: Vec<_> = survey.packages.iter().collect();
    packages.sort_by(|a, b| b.1.cmp(a.1));
    println!(
        "packages:    {} distinct, top {:?}",
        packages.len(),
        &packages[..packages.len().min(10)]
    );
    println!("key kinds:   {:?}", survey.key_kinds);
    println!(
        "size:        {:.1} MB of values as dumped; as a corpus {:.1} MB pooled per {}-record \
         shard + {:.1} MB for {} distinct modules",
        mb(survey.value_bytes),
        mb(survey.pooled_value_bytes),
        args.shard_size,
        mb(survey.module_bytes),
        survey.distinct_modules
    );
    Ok(())
}

/// The most versions one `capture --begin-version ... --end-version` takes (80 MB of version
/// numbers): weeks of capturing at a few transactions a second, far more than one run gets through.
const MAX_CAPTURE_RANGE: u64 = 10_000_000;

fn capture(args: CaptureArgs) -> Result<()> {
    let versions: Vec<u64> = match (args.versions_file, args.begin_version, args.end_version) {
        (Some(path), None, None) => std::fs::read_to_string(&path)?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.parse())
            .collect::<Result<_, _>>()?,
        (None, Some(begin), Some(end)) => {
            anyhow::ensure!(end >= begin, "--end-version must be >= --begin-version");
            // The versions are listed up front: a range past this would not fit in memory.
            anyhow::ensure!(
                end - begin < MAX_CAPTURE_RANGE,
                "the range holds more than {} versions; capture it in parts",
                MAX_CAPTURE_RANGE
            );
            (begin..=end).collect()
        },
        _ => anyhow::bail!(
            "provide either --versions-file, or both --begin-version and --end-version"
        ),
    };
    let network: Network = args.network.parse().map_err(anyhow::Error::msg)?;
    let (manifest, report) = capture::run(capture::CaptureConfig {
        base_url: network.into(),
        api_key: args.api_key,
        versions,
        out: args.out.clone(),
        corpus_id: args.corpus_id,
        network: args.network,
        shard_size: args.shard_size,
        concurrency: args.concurrency,
        complete: args.complete,
        limits: args.limits.limits()?,
    })?;
    print_written(&manifest, &args.out, &report)
}

fn import(args: ImportArgs) -> Result<()> {
    let framework = match (args.framework, args.offline) {
        (_, true) | (FrameworkArg::Head, false) => FrameworkSource::Head,
        (FrameworkArg::Onchain, false) => FrameworkSource::OnChain,
    };
    let chain = if args.offline {
        None
    } else {
        let network: Network = args.network.parse().map_err(anyhow::Error::msg)?;
        Some(RestChainSource::new(
            network.into(),
            args.api_key.clone(),
            args.concurrency,
        )?)
    };
    let config = ImportConfig {
        legacy_dump: args.legacy_dump,
        out: args.out,
        corpus_id: args.corpus_id,
        network: args.network,
        framework,
        shard_size: args.shard_size,
        limit: args.limit,
        cache_dir: args.cache_dir,
        complete: !args.skip_completion,
        limits: args.limits.limits()?,
    };
    let (manifest, report) = import::import(
        &config,
        chain.as_ref().map(|c| c as &dyn import::ChainSource),
    )?;
    if report.without_features > 0 {
        eprintln!(
            "warning: {} records lack the chain's feature flags (no state completion) and will \
             compare as incomplete",
            report.without_features
        );
    }
    print_written(&manifest, &config.out, &report)
}

fn aggregate(args: AggregateArgs) -> Result<()> {
    let (files, results) = aggregate::read_all_results(&args.results)?;
    // The outputs must not be an input (a results file is read before they are written, so it
    // would be lost), nor one another, by path or through a link.
    let outputs: Vec<&std::path::Path> = [&args.summary_out, &args.markdown]
        .into_iter()
        .flatten()
        .map(|path| path.as_path())
        .collect();
    for (n, output) in outputs.iter().enumerate() {
        ensure_not_dangling(output)?;
        for other in files
            .iter()
            .map(|file| file.as_path())
            .chain(outputs[..n].iter().copied())
        {
            anyhow::ensure!(
                !same_file(output, other),
                "--summary-out and --markdown must differ from each other and from the results: {:?}",
                output
            );
        }
    }
    let summary = comparison::Summary::new(&results);
    let failures = aggregate::failures(&summary);
    if let Some(path) = &args.summary_out {
        std::fs::write(path, serde_json::to_vec_pretty(&summary)?)?;
    }
    let report = aggregate::markdown(&summary, &failures);
    if let Some(path) = &args.markdown {
        // Appends, so several runs can report into one file.
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        file.write_all(report.as_bytes())?;
    }
    println!(
        "{} result file(s), {} record(s)\n{}",
        files.len(),
        summary.records,
        report
    );
    anyhow::ensure!(failures.is_empty(), "{}", failures.join("; "));
    Ok(())
}

fn targets(args: TargetsArgs) -> Result<()> {
    anyhow::ensure!(
        args.to_version >= args.from_version,
        "--to-version must be >= --from-version"
    );
    let versions = targets::select(
        &targets::GraphQlIndexer::new(args.indexer_url, args.api_key),
        args.from_version,
        args.to_version,
        args.per_function,
    )?;
    let mut out = BufWriter::new(File::create(&args.out)?);
    for version in &versions {
        writeln!(out, "{}", version)?;
    }
    out.flush()?;
    println!("wrote {} versions to {:?}", versions.len(), args.out);
    Ok(())
}

fn compare(args: CompareArgs) -> Result<()> {
    let limits = args.limits.limits()?;
    let corpus = Corpus::open(&args.corpus)?;
    if let Ok(stopped) = std::fs::read_to_string(args.corpus.join(corpus::STOPPED_FILE)) {
        eprintln!(
            "warning: this corpus is partial, its capture or import stopped early: {}",
            stopped.trim()
        );
    }
    // Each shard once, in order, however often it was named.
    let shards: BTreeSet<usize> = if args.shards.is_empty() {
        (0..corpus.manifest().shards.len()).collect()
    } else {
        args.shards.into_iter().collect()
    };
    let versions: HashSet<u64> = args.versions.into_iter().collect();
    let mut out = open_out(args.out.as_deref(), &args.corpus, false)?;
    let mut summary = comparison::SummaryBuilder::default();
    let mut found = HashSet::new();
    for n in shards {
        if !versions.is_empty() && !shard_may_hold(&corpus, n, versions.iter()) {
            continue;
        }
        let shard = corpus.load_shard(n)?;
        for record in &shard.records {
            if !versions.is_empty() && !versions.contains(&record.version) {
                continue;
            }
            found.insert(record.version);
            let result = comparison::compare_record(&corpus, &shard, record, &limits)?;
            writeln!(out, "{}", serde_json::to_string(&result)?)?;
            summary.add(&result);
        }
    }
    out.flush()?;
    eprintln!("{}", serde_json::to_string_pretty(&summary.finish())?);
    // A requested version that was not compared must not pass for one that was.
    let mut missing: Vec<_> = versions.difference(&found).collect();
    missing.sort();
    anyhow::ensure!(
        missing.is_empty(),
        "versions not in the corpus (or not in the --shards compared): {:?}",
        missing
    );
    // Only a complete run takes `--out`.
    out.finish()
}

fn replay(args: ReplayArgs) -> Result<()> {
    let limits = args.limits.limits()?;
    let corpus = Corpus::open(&args.corpus)?;
    let mut remaining: BTreeSet<u64> = args.versions.into_iter().collect();
    let mut out = open_out(args.out.as_deref(), &args.corpus, true)?;
    let replayed = (|| -> Result<()> {
        for n in 0..corpus.manifest().shards.len() {
            if remaining.is_empty() {
                break;
            }
            if !shard_may_hold(&corpus, n, remaining.iter()) {
                continue;
            }
            let shard = corpus.load_shard(n)?;
            for record in &shard.records {
                if remaining.remove(&record.version) {
                    replay::replay(
                        &corpus,
                        &shard,
                        record,
                        args.vm.runs_v1(),
                        args.vm.runs_v2(),
                        &limits,
                        &mut out,
                    )?;
                    writeln!(out)?;
                }
            }
        }
        anyhow::ensure!(
            remaining.is_empty(),
            "versions not in the corpus: {:?}",
            remaining
        );
        Ok(())
    })();
    // Even a replay that failed keeps what it printed: it is for triage. Its own error comes first.
    let finished = out.finish();
    replayed.and(finished)
}

/// An `--out` file, or stdout. `compare`'s file is written under a temporary name next to it and takes
/// its name only at [`Out::finish`]: a run that fails before then leaves `--out` as it was, rather
/// than truncated or holding a prefix of the results that `aggregate` would take for a whole run.
struct Out {
    writer: Box<dyn Write>,
    /// The temporary file and the `--out` it becomes, until it does.
    pending: Option<(std::path::PathBuf, std::path::PathBuf)>,
}

impl Out {
    fn finish(mut self) -> Result<()> {
        self.writer.flush()?;
        if let Some((tmp, path)) = &self.pending {
            std::fs::rename(tmp, path)
                .with_context(|| format!("failed to move {:?} to {:?}", tmp, path))?;
            // Moved: nothing is left to remove.
            self.pending = None;
        }
        Ok(())
    }
}

impl Write for Out {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        if let Some((tmp, _)) = &self.pending {
            let _ = std::fs::remove_file(tmp);
        }
    }
}

/// Where `compare` and `replay` write: `path`, or stdout. Never inside `corpus`: the file would
/// replace whatever corpus file it names, after the corpus was opened.
fn open_out(
    path: Option<&std::path::Path>,
    corpus: &std::path::Path,
    as_written: bool,
) -> Result<Out> {
    Ok(match path {
        Some(path) => {
            ensure_not_dangling(path)?;
            // An existing link is written through, as before: the file it names takes the results,
            // so the checks and the temporary file are about that file rather than the link.
            let resolved = if path.is_symlink() {
                path.canonicalize()
                    .with_context(|| format!("failed to resolve --out {:?}", path))?
            } else {
                path.to_path_buf()
            };
            let path = resolved.as_path();
            anyhow::ensure!(
                !path.is_dir(),
                "--out {:?} is a directory; name a file",
                path
            );
            // A character device (`/dev/null`, a terminal) cannot be replaced: it is written in place,
            // as it holds nothing a failed run could spoil. Other special files (a pipe, which would
            // block until read, or a block device) are refused.
            if let Ok(metadata) = std::fs::metadata(path)
                && !metadata.is_file()
            {
                use std::os::unix::fs::FileTypeExt;
                anyhow::ensure!(
                    metadata.file_type().is_char_device(),
                    "--out {:?} is neither a regular file nor a character device",
                    path
                );
                return Ok(Out {
                    writer: Box::new(BufWriter::new(
                        std::fs::OpenOptions::new()
                            .write(true)
                            .open(path)
                            .with_context(|| format!("failed to open {:?}", path))?,
                    )),
                    pending: None,
                });
            }
            anyhow::ensure!(
                !mono_move_replay::import::is_within(path, corpus)?
                    && !is_corpus_file(path, corpus)?,
                "--out {:?} is inside the corpus {:?}",
                path,
                corpus
            );
            // `replay`'s output is for triage: written as it goes, so a crash keeps what came before.
            if as_written {
                return Ok(Out {
                    writer: Box::new(BufWriter::new(
                        File::create(path)
                            .with_context(|| format!("failed to create {:?}", path))?,
                    )),
                    pending: None,
                });
            }
            let name = path
                .file_name()
                .with_context(|| format!("--out {:?} names no file", path))?;
            let tmp = path.with_file_name(format!(
                ".{}.{}.tmp",
                name.to_string_lossy(),
                std::process::id()
            ));
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .with_context(|| format!("failed to create {:?}", tmp))?;
            Out {
                writer: Box::new(BufWriter::new(file)),
                pending: Some((tmp, path.to_path_buf())),
            }
        },
        None => Out {
            writer: Box::new(std::io::stdout().lock()),
            pending: None,
        },
    })
}

/// Refuses an output `path` that is a link to nothing: writing it would create the link's target,
/// which the checks on outputs cannot see.
fn ensure_not_dangling(path: &std::path::Path) -> Result<()> {
    anyhow::ensure!(
        !path.is_symlink() || path.exists(),
        "output {:?} is a link to nothing",
        path
    );
    Ok(())
}

/// Whether shard `n` may hold any of `versions`, from the version range the manifest gives it, so
/// that a shard holding none is not loaded. A shard the manifest lacks is loaded, to fail there.
fn shard_may_hold<'a>(
    corpus: &Corpus,
    n: usize,
    mut versions: impl Iterator<Item = &'a u64>,
) -> bool {
    match corpus.manifest().shards.get(n) {
        Some(shard) => {
            versions.any(|version| (shard.first_version..=shard.last_version).contains(version))
        },
        None => true,
    }
}

/// What `capture` and `import` print once the corpus is written.
fn print_written(
    manifest: &corpus::Manifest,
    out: &std::path::Path,
    report: &impl serde::Serialize,
) -> Result<()> {
    println!(
        "wrote corpus {:?} to {:?}: {} records in {} shards, {} modules, {} frameworks",
        manifest.corpus_id,
        out,
        manifest.num_records(),
        manifest.shards.len(),
        manifest.num_modules,
        manifest.num_frameworks
    );
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

/// Whether `path` is an existing file that is also one of `corpus`'s files, through a hard link
/// that [`mono_move_replay::import::is_within`] cannot see: creating it would truncate that file.
fn is_corpus_file(path: &std::path::Path, corpus: &std::path::Path) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let Ok(target) = std::fs::metadata(path) else {
        return Ok(false);
    };
    let mut files = vec![];
    for dir in [corpus.to_path_buf(), corpus.join(corpus::SHARDS_DIR)] {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            files.extend(
                entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path()),
            );
        }
    }
    for file in files {
        // An entry that cannot be inspected cannot be `path` either.
        let Ok(metadata) = std::fs::metadata(&file) else {
            continue;
        };
        if (metadata.dev(), metadata.ino()) == (target.dev(), target.ino()) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether `a` and `b` name the same file: the same path, or, if both exist, the same file through
/// a link.
fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => (a.dev(), a.ino()) == (b.dev(), b.ino()),
        (Ok(_) | Err(_), Ok(_) | Err(_)) => resolved(a) == resolved(b),
    }
}

/// `path`, which may not exist yet, with its directory resolved, so that `..` and links among the
/// directories do not tell two names of the same file apart.
fn resolved(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let path = std::path::absolute(path).ok()?;
    let dir = path.parent().and_then(|dir| dir.canonicalize().ok());
    match (dir, path.file_name()) {
        (Some(dir), Some(name)) => Some(dir.join(name)),
        (Some(_) | None, Some(_) | None) => Some(path),
    }
}
