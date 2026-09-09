# Perf tools

Scripts used to produce `../perf_log.md`. They measure MonoMove against the
legacy MoveVM on identical recorded blocks, and read `samply` profiles.

Everything takes its scratch directory from `$MONOPROF_WORK`. Set it once:

```bash
export MONOPROF_WORK=$TMPDIR/monoprof
mkdir -p "$MONOPROF_WORK"
```

## Measuring

```bash
cargo build --profile performance -p aptos-executor-benchmark --bin aptos-executor-benchmark
./setup.sh                                   # create the db, record 30 blocks per workload
./ab.sh 4 base=$PWD/../../../../../target/performance/aptos-executor-benchmark
```

`setup.sh` is idempotent and skips work that already exists. It records 30
blocks of 500 transactions for `bench-clob` and `bench-aave`, which is the
14030 transactions every profile number is divided by.

`ab.sh <reps> <label>=<abs-path-to-bin> ...` runs each arm against both
workloads, both VMs, interleaved rep by rep, and prints a table. Binary paths
must be absolute — the script `cd`s to the repo root. It **appends** to
`$MONOPROF_WORK/ab.tsv`, so move that file aside before starting a new sweep.

`report.py <tsv> [labels...]` re-prints the table from a saved tsv.

### Rules that make the numbers mean anything

- Build every arm as a separate binary with identical flags, including debug
  info. A binary that differs only in debug info measures differently.
- Report the **max** over reps, not the median. Interference on this machine is
  one-sided: it only ever makes a run slower, and the workload is a
  deterministic replay.
- Only compare rows measured in the same sweep. The legacy arm is the built-in
  control and should stay flat across arms; if it moves, the sweep is bad.
- Never run other CPU work while a sweep is live. A background `cargo build`
  invalidates the whole sweep.
- The noise floor is about 1.5%. A change below that is not a result.
- `ab.sh` and `samply record` need the Bash sandbox disabled, because the
  benchmark binds a port.

## Profiling

```bash
./prof.sh    # records both workloads x both VMs into $MONOPROF_WORK
```

Or directly:

```bash
samply record --save-only --unstable-presymbolicate --rate 4999 \
  -o "$MONOPROF_WORK/<label>-<workload>-<arm>.json.gz" -- <bin> <args>
```

Each profile has a sibling symbol file named `<name>.json.syms.json` — note the
doubled `.json`. Every analysis script takes the profile and the symbol file as
its first two arguments.

| script | answers |
|---|---|
| `incl3.py <prof> <syms> name=regex...` | CPU-weighted inclusive cost of named subtrees. Regions may overlap; each is reported independently. |
| `under.py <prof> <syms> <ancestor-rx> [limit] [--exclude=rx]` | Self-cost ranking restricted to samples passing through an ancestor. The workhorse. |
| `blame.py <prof> <syms> <leaf-rx> [limit]` | Charges a leaf class to its nearest non-matching ancestor. Answers "who is doing all this malloc". |
| `callers.py <prof> <syms> <thread> <pat> <depth>` | Top ancestor chains for samples whose leaf matches a pattern. |
| `part.py <prof> <syms>` | Exclusive partition of the thread by leaf class. Buckets sum to the total. |
| `plumb.py <prof> <syms>` | How much of the thread is allocator, hashing, memcpy, and clocks rather than requested work. |
| `an3.py` | Shared symbol loader. Not run directly. |

Set `NTXN` if the block count differs from the default 14030.

### Rules for reading a profile

These were learned the expensive way. All of them are in `../perf_log.md` with
the incidents that produced them.

- Take only the **largest** `txn_executor` thread. The others are idle and
  dilute everything.
- Weight by `threadCPUDelta` (microseconds), not sample count.
  `an3.collect()` sums sample counts across all threads — do not use it for
  microseconds.
- **Profiles over-state short hot leaves by 2 to 3x.** A leaf that profiles at
  4% is often worth 1.5% when you delete it.
- **Blame by caller before believing a growth number.** A symbol getting more
  expensive is usually a caller calling it more often.
- **Trust a ceiling arm over the profile.** Before optimizing a cost, build an
  arm that removes it entirely and measure that. It bounds the payoff before
  any real work happens. Twice in the log this showed the payoff was much
  smaller than the profile suggested.
- Profiles are versioned by experiment. A mono profile recorded before an
  experiment landed does not describe the code after it. Legacy profiles are
  not versioned, because the legacy path never changed.

### Scaling profile numbers to measured ones

Profiled microseconds per transaction run high. The observed factors:

| profile | profiled us/txn | measured us/txn | factor |
|---|---|---|---|
| clob mono | 73.54 | 63.0 | 0.857 |
| clob legacy | 719.59 | 514.7 | 0.715 |
