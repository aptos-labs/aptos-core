# Replay comparison: user guide

This guide shows how to use `mono-move-replay` to compare MonoMove against AptosVM on
real transactions, and how to read what it reports. It follows one real run from start to finish.
How the tool decides its verdicts, and why they can be trusted, is in the
[design doc](../../docs/replay_comparison_design.md).

> **About the example run.** Every output below is real, from a run on 2026-10-08 against
> mainnet. Capture outputs and timings are from a rerun on 2026-10-09, with the current defaults. The numbers describe MonoMove at that date. When you run it, expect the same shape but
> different counts as MonoMove gains features and bugs are fixed. Long lines are cut with `…`.

## Contents

1. [Concepts](#1-concepts)
2. [Setup](#2-setup)
3. [Walkthrough: new mainnet data](#3-walkthrough-new-mainnet-data)
4. [Reading the summary](#4-reading-the-summary)
5. [Reading the per-record results](#5-reading-the-per-record-results)
6. [Triage: from a mismatch to a bug report](#6-triage-from-a-mismatch-to-a-bug-report)
7. [Merging runs](#7-merging-runs)
8. [Legacy dumps](#8-legacy-dumps)
9. [Practical notes](#9-practical-notes)
10. [FAQ](#10-faq)

---

## 1. Concepts

| Term | Meaning |
|---|---|
| **V1** | The legacy AptosVM. It is the reference. |
| **V2** | MonoMove, through the MonoMove-backed Aptos transaction executor. |
| **Corpus** | A directory of captured transactions, each stored with the state it read on chain (plus its full module closure, and with state completion whatever either VM reads beyond), its on-chain status and its auxiliary info (both missing only from offline imports). Written once, never changed. |
| **Record** | One transaction in a corpus. |
| **Verdict** (`outcome`) | What comparing one record found: whether the VMs agree, and if not, why. |
| **Known difference** | A difference MonoMove is known to have, a documented TODO: it does not emit state-value metadata yet, and it writes entries whose bytes did not change. Records that differ only like this count as agreeing. |
| **Gap** | A feature MonoMove reports it does not have yet, such as a missing native. Not a bug, but the record cannot be compared. |

The workflow has three steps. Choose transactions, capture them into a corpus, then compare. The
corpus is the input; the verdicts are the output. A corpus can be compared any number of times,
for example after a MonoMove change, without fetching anything again.

---

## 2. Setup

```bash
cargo build --release -p mono-move-replay
B=target/release/mono-move-replay
```

Everything below is run from the repository root. Network access is needed only for `targets`,
`capture` and `import`. `compare`, `replay` and `aggregate` work offline on a corpus.

---

## 3. Walkthrough: new mainnet data

### 3.1 Choose transactions (`targets`, optional)

A contiguous range of versions is dominated by a few busy contracts, such as oracle updates and
DEX keepers. Instead, `targets` asks the indexer for every entry function called in a window, then
picks the most recent calls of each:

```console
$ $B targets --from-version 7509350000 --to-version 7509372000 --per-function 3 --out versions.txt
wrote 126 versions to "versions.txt"            # 13 s
$ head -3 versions.txt
7509350268
7509350898
7509351517
```

- **`--from-version` / `--to-version`:** the window. The calls picked come from inside it.
- **`--per-function 3`:** at most 3 transactions per entry function, the most recent ones. The file
  holds about (distinct entry functions) × 3 versions, fewer when a function was called less often.
  A higher value samples more paths through each function, such as a success and an abort, or a
  `None` and a `Some` argument. A lower value buys more breadth for the cost.
- Scripts have no entry function, so they are never picked.

You can skip this step and capture a plain range instead (§3.2).

### 3.2 Capture a corpus (`capture`)

```console
$ $B capture --versions-file versions.txt --out corpus-targeted --corpus-id mainnet-targeted
wrote corpus "mainnet-targeted" to "corpus-targeted": 126 records in 1 shards, 1115 modules, 1 frameworks
{
  "captured": 126,
  "skipped": {},
  "completed_values": 0,
  "completed_absent": 0,
  "v2_crashes": 0
}                                               # about 1.5 min
```

For a contiguous range, pass `--begin-version` and `--end-version` instead:

```console
$ $B capture --begin-version 7509371328 --end-version 7509371427 --out corpus-fresh --corpus-id mainnet-fresh
wrote corpus "mainnet-fresh" to "corpus-fresh": 100 records in 1 shards, 320 modules, 1 frameworks
{ "captured": 100, "skipped": {}, "completed_values": 0, "completed_absent": 0, "v2_crashes": 0 }   # 35 s
```

What the report says:

| Field | Meaning |
|---|---|
| `captured` | Records written. |
| `skipped` | Transactions not captured, by reason, for example: not on chain, genesis, a kind that is not replayed, no package registry at `0x1`, a framework module outside its release, a capture that failed. Failed captures are also printed, as `version N: skip: …`. |
| `completed_values` / `completed_absent` | State that *state completion* added to the records kept: values, and keys found absent, that a replay reads beyond what V1 read on chain. Only with `--complete`, which replays both VMs to find them; 0 otherwise. On fresh mainnet captures it has so far added nothing. |
| `v2_crashes` | With `--complete`: records where MonoMove crashed during completion. They are **kept**, so `compare` reports them as `mono_crash`. For example, version 7509371416 hits a known MonoMove bug (#20699), so with `--complete` it is counted here. |

Rules worth knowing:

- **`--out` must be a new or empty directory.** A corpus never overwrites another. It may not be a
  symlink, or have a `..` after a directory name (a leading `..` is fine). An empty directory
  that already exists becomes the run's: nothing else should write into it while the run lasts.
- **Interrupted runs.** If a run stops partway on an error (for example the node goes away), the
  records captured so far are kept in `<out>.partial`, with a `STOPPED` file saying why, and `--out`
  stays free for a rerun. A process that is killed (Ctrl-C, an out-of-memory kill) keeps nothing
  usable: remove its `--out` and run again.
- **Network.** `--network` defaults to mainnet; `testnet`, `devnet` or a REST URL also work. Add
  `--api-key <KEY>` to avoid the anonymous rate limit.

### 3.3 Compare (`compare`)

```console
$ $B compare --corpus corpus-targeted --out compare-targeted.jsonl      # 11 s
{
  "records": 126,
  "matches": 0,
  "known_differences": 46,
  "mismatches": 11,
  "not_candidates": 0,
  "not_comparable": 0,
  "incomplete": {},
  "unsupported": 69,
  "mono_crashes": 0,
  "v1_errors": 0,
  "skipped": 0,
  "unchecked": 0,
  "overrides": { "integer_supply": 23, "zero_gas": 126 },
  "unsupported_kinds": [
    [ { "kind": "missing_native", "module": "1::function_info", "function": "load_function_impl" }, 56 ],
    [ { "kind": "runtime_unsupported", "what": "function values are not yet supported" }, 12 ],
    [ { "kind": "missing_native", "module": "1::string_utils", "function": "native_format" }, 1 ]
  ],
  "versions_in_several_corpora": 0
}
```

`compare` writes two things:

- **The summary**, on stderr: the totals above (§4).
- **One verdict per record**, one JSON line each, in the `--out` file, or on stdout without `--out`
  (§5). `--out` (here and for `replay`) may not be inside the corpus, a hard link to a corpus
  file, a symlink to nothing, a directory, a pipe or a block device. `compare`'s file takes the
  results only once the run completes (a failed run leaves it as it was); `replay` writes as it
  goes; `/dev/null` and terminals are written directly.

The contiguous range gives:

```console
$ $B compare --corpus corpus-fresh --out compare-fresh.jsonl            # 9 s
{ "records": 100, "matches": 0, "known_differences": 97, "mismatches": 0, "unsupported": 2,
  "mono_crashes": 1, "incomplete": {}, … }
```

The contrast between the two corpora shows why `targets` exists. The 100 consecutive transactions
are mostly the same few contracts, and they agree. The 126 targeted ones reach 11 real mismatches
and many more gaps.

---

## 4. Reading the summary

The verdict counts add up to `records`. In the targeted run: 46 + 11 + 69 = 126.

| Field | Targeted run | Is it a problem? | Meaning |
|---|---|---|---|
| `matches` | 0 | no | Outputs byte-identical. **0 is normal today**: agreeing records almost always show a known difference (next row). |
| `known_differences` | 46 | no, a pass | The VMs agree on what the transaction did. The outputs differ only in MonoMove's known differences (§1). |
| `mismatches` | 11 | **yes, a test failure** | A real divergence (a different status, write, native-position write, hot-state key, event or gas), or MonoMove failing to set up, panicking, or failing to write its output. See §6. |
| `mono_crashes` | 0 | **yes, a test failure** | MonoMove ran out of memory or time, read more than a million keys, or died. |
| `unsupported` | 69 | no, a gap | MonoMove reported a feature it lacks. The record could not be compared. |
| `incomplete` | {} | should be empty | A VM read state the corpus never observed, counted per VM (`v1`, `v2`). Non-empty means some verdicts could not be decided. It is not a failure, so `aggregate` does not fail on it (an offline import is expected to have some), but those records are untested. Recapturing usually fixes it. |
| `not_candidates` | 0 | no | V1, on the patched state, did not reproduce the on-chain status, so the record was excluded. |
| `not_comparable` | 0 | no | The transaction stopped at a gas or metering limit, on chain or in V1, which unmetered MonoMove cannot hit. One that stopped on chain is not replayed at all (`capture` skips it). Also a call to a native neither VM has: V1 fails with `MISSING_DEPENDENCY` where MonoMove reports the native missing. |
| `v1_errors` | 0 | investigate | V1 failed or panicked, or the record's patched state could not be prepared. That is a problem with the replay setup, not with MonoMove. |
| `skipped` | 0 | no | A transaction kind that is not replayed. |
| `unchecked` | 0 | — | Records with no on-chain status to check against (only offline imports). |
| `overrides` | | — | How many records each state patch applied to. `zero_gas` applies to every record. `integer_supply` rewrites the APT supply as a plain integer in the records that touch it. |
| `unsupported_kinds` | | — | MonoMove's gaps, most frequent first. In this run, 56 of the 69 come from one missing native, `0x1::function_info::load_function_impl`, which every dispatchable fungible-asset transfer needs. |
| `versions_in_several_corpora` | 0 | — | Only meaningful after `aggregate` merges corpora (§7). |

**A run reads well if:**
1. `incomplete` is empty, so every verdict was decided.
2. `mismatches` and `mono_crashes` are 0, or each one is a known, filed bug.
3. Everything else is a pass (`known_differences`), a gap (`unsupported`), or excluded with a
   reason (`not_candidates`, `not_comparable`).

So the targeted run reads: of the 57 records MonoMove could run (46 + 11), 46 agree with AptosVM.
The 11 that do not trace to three bugs (§6). Most of the rest is one missing native. No verdict is
in doubt.

---

## 5. Reading the per-record results

Each line of `compare-targeted.jsonl` is one record:

| Field | Meaning |
|---|---|
| `corpus_id`, `version` | Which record. |
| `label` | The function called, or the transaction kind. |
| `overrides` | The state patches that applied to it. |
| `onchain_checked` | Whether the record has an on-chain status to filter candidates by (false only for offline imports). |
| `outcome` | The verdict, plus the fields that verdict carries (below). |

A real line for each kind of verdict:

**`match`** carries nothing beyond the common fields.

**`known_difference`** carries `diffs`, every difference, each one marked `(known)`:
```json
{"version":7509371328,"label":"0x50ead…::dex_accounts_entry::place_bulk_orders_to_subaccount_with_repricing",
 "outcome":"known_difference","diffs":[
  "write to …0x1::nonce_validation::NonceHistory in V2 only, with the bytes it already held (known)",
  "write to …perp_market::PerpMarket carries metadata in V1 only (known)", …]}
```

**`mismatch`** carries `diffs`. The first entry is usually the decisive one:
```json
{"version":7509351802,"label":"0x1c32…::panora_swap::router_entry<…>","outcome":"mismatch","diffs":[
  "statuses differ: V1=Keep(MoveAbort { location: 0x1c32…::panora_swap_aggregator_fungible_asset, code: 2,
   info: Some(AbortInfo { reason_name: \"E_OUTPUT_LESS_THAN_MINIMUM_TRY_INCREASING_SLIPPAGE\" … }) }),
   V2=Keep(MiscellaneousError(Some(INVALID_MAIN_FUNCTION_SIGNATURE)))",
  "write to …0x1::nonce_validation::NonceHistory in V2 only, with the bytes it already held (known)"]}
```

**`mono_unsupported`** carries `kind`, the gap, and `v1_status`, what V1 did:
```json
{"version":7509371378,"label":"0x1::primary_fungible_store::transfer<0x1::fungible_asset::Metadata>",
 "outcome":"mono_unsupported","kind":{"kind":"missing_native","module":"1::function_info",
 "function":"load_function_impl"},"v1_status":"Keep(Success)"}
```

**`mono_crash`** carries `reason` and `v1_status`, and for triage up to 20 of the unobserved keys the
crash read (`unobserved`) with their total (`unobserved_total`):
```json
{"version":7509371416,"label":"0x50ead…::admin_apis::update_mark_with_blended_chainlink_feeds",
 "outcome":"mono_crash","reason":"MonoMove exceeded the memory limit (4096 MB)","v1_status":"Keep(Success)",
 "unobserved":[],"unobserved_total":0}
```

**`not_candidate`** carries `onchain` and `v1`, the two statuses that differ:
```json
{"version":1457409,"label":"0x1::aptos_account::transfer","outcome":"not_candidate",
 "onchain":"Keep(MoveAbort { location: 0x1::transaction_validation, code: 132077, info: None })",
 "v1":"Keep(Success)"}
```
This one aborted on chain in the epilogue, because it could not pay for gas. With gas zeroed for
the replay, V1 succeeds, so the record cannot be compared.

**`incomplete`** carries `vm` and the `unobserved` keys. **`not_comparable`** carries `v1_status`.
**`v1_error`** carries `detail`. **`skipped`** carries `reason`.

Useful queries:

```bash
F=compare-targeted.jsonl
jq -r .outcome $F | sort | uniq -c                                    # count by verdict
jq -r 'select(.outcome=="mismatch") | "\(.version) \(.label|split("<")[0])\n    \(.diffs[0])"' $F
jq 'select(.version==7509351802)' $F                                   # one record in full
jq -r 'select(.outcome=="mono_unsupported") | .kind | tostring' $F | sort | uniq -c   # gaps
jq -r 'select(.kind.function=="load_function_impl") | .label|split("<")[0]' $F | sort | uniq -c
```

---

## 6. Triage: from a mismatch to a bug report

### 6.1 Group the mismatches

```console
$ jq -r 'select(.outcome=="mismatch") | "\(.version) \(.label|split("<")[0]|split("::")[1:]|join("::"))"' compare-targeted.jsonl
7509351802 panora_swap::router_entry
7509352191 panora_swap::router_entry
7509357628 aurora::panora_swap
…
7509370729 tiered_oracle::update_price
```

Mismatches with the same function and the same first diff usually share a cause. In this run the
11 fall into three groups.

### 6.2 Look at one record on both VMs (`replay`)

`replay` prints each VM's output: status, gas, the write set (key, kind, metadata, bytes) and the
events. Native-position writes and hot-state keys are compared but not printed. It runs on the same patched state `compare` used.

```console
$ $B replay --corpus corpus-targeted --versions 7509351802 --vm both
version: 7509351802
transaction: 0x1c32…::panora_swap::router_entry<0x1::aptos_coin::AptosCoin, …>
on chain: status MoveAbort { location: 0x1c32…::panora_swap_aggregator_fungible_asset, code: 2, … }
overrides: ["zero_gas", "integer_supply"]

== V1 (AptosVM)
status: Keep(MoveAbort { location: 0x1c32…::panora_swap_aggregator_fungible_asset, code: 2,
        info: Some(AbortInfo { reason_name: "E_OUTPUT_LESS_THAN_MINIMUM_TRY_INCREASING_SLIPPAGE" … }) })
gas used: 0
write set: 1 write(s)
  StateKey::TableItem { handle: 16e1f82a…, key: d362000000000000 }
    modification, 408 byte(s), …
events: 1
  v2 0x1::transaction_fee::FeeStatement: 0x0000…

== V2 (MonoMove)
status: Keep(MiscellaneousError(Some(INVALID_MAIN_FUNCTION_SIGNATURE)))
gas used: 0
write set: 2 write(s)
  StateKey::AccessPath { address: 0x1, path: "Resource(0x1::nonce_validation::NonceHistory)" } …
  StateKey::TableItem { handle: 16e1f82a…, key: d362000000000000 } …
events: 1
  v2 0x1::transaction_fee::FeeStatement: 0x0000…
```

How to read it:
- V1 reproduces the chain: the swap aborts on slippage, inside the swap itself.
- MonoMove never reaches the swap: it rejects the call at the signature check.
- The extra `NonceHistory` write is a known difference (unchanged bytes), not part of the bug.

So the question is why MonoMove rejects this signature. The function's second parameter is
`Option<signer>`. V1 checks only the argument's *value*, and this transaction passes `None`.
MonoMove checks the *type*, and `signer` is not an allowed argument type.

`--vm v1` or `--vm v2` prints one VM only. With `--vm v2`, V1 still runs first, unprinted, so that
MonoMove is not run on a transaction V1 stops at a metering limit (see the FAQ).

### 6.3 The three groups in this run

| Issue | Records | What differs |
|---|---|---|
| [MOVE-198](https://linear.app/aptoslabs/issue/MOVE-198): `Option<signer>` parameter rejected | 6, panora `router_entry` and `aurora::panora_swap` | MonoMove: `INVALID_MAIN_FUNCTION_SIGNATURE`. V1 / chain: success, or an abort later in the swap. |
| [MOVE-199](https://linear.app/aptoslabs/issue/MOVE-199): missing `Object<T>` argument | 2, `place_twap_order_to_subaccount` | MonoMove: `FAILED_TO_DESERIALIZE_ARGUMENT`. V1 / chain: an abort in `0x1::object`. |
| [MOVE-200](https://linear.app/aptoslabs/issue/MOVE-200): no `AbortInfo` | 3, `cancel_tp_sl_order_for_position` and `tiered_oracle::update_price` | Same abort, but MonoMove's carries `info: None`. |

A bug report needs three things:
- the versions;
- the two statuses or diffs, from the JSONL line;
- the `replay --vm both` output.

The issues above are written that way. Any record can be recaptured on its own, with
`--begin-version N --end-version N`, for a minimal repro.

### 6.4 A crash

```console
$ $B replay --corpus corpus-fresh --versions 7509371416 --vm v2
version: 7509371416
transaction: 0x50ead…::admin_apis::update_mark_with_blended_chainlink_feeds (user/entry_function)
on chain: status Success, gas 1208
overrides: ["zero_gas"]

== V2 (MonoMove)
crashed: MonoMove exceeded the memory limit (4096 MB)
```

MonoMove runs in a child process held to a memory limit and a time limit, so a runaway bug is
contained instead of killing the run: `replay` prints it, `compare` reports it as a `mono_crash`
verdict, and state completion (in `import`, and `capture --complete`) keeps the record and counts
it in `v2_crashes`. This one is #20699 (MOVE-192): MonoMove's specializer never finishes lowering
one function on this path.

### 6.5 Rerunning selected records

After a MonoMove fix, rerun only the affected records:

```console
$ $B compare --corpus corpus-fresh --versions 7509371400 --versions 7509371416
{"corpus_id":"mainnet-fresh","version":7509371400,"label":"block_epilogue_v1",…,"outcome":"known_difference",…}
{"corpus_id":"mainnet-fresh","version":7509371416,…,"outcome":"mono_crash","reason":"MonoMove exceeded the memory limit (4096 MB)",…}
{ "records": 2, "known_differences": 1, "mono_crashes": 1, … }
```

Without `--out`, the verdict lines go to stdout. A requested version the corpus does not hold is an error, after
the others are compared, so a mistyped version cannot pass for one that was checked.

---

## 7. Merging runs

`aggregate` merges any number of `compare` results, for example one per corpus or per shard:

```console
$ $B aggregate --results compare-fresh.jsonl compare-targeted.jsonl compare-legacy.jsonl \
    --summary-out summary.json --markdown report.md
3 result file(s), 276 record(s)
## MonoMove replay comparison: failed

- **11 mismatch(es)**
- **1 MonoMove crash(es)**

| Outcome | Records |
|---|---|
| match | 0 |
| known difference | 143 |
| mismatch | 11 |
| MonoMove unsupported | 119 |
| MonoMove crash | 1 |
| incomplete | 0 |
| not candidate | 2 |
| … | … |
| total | 276 |

### MonoMove gaps by frequency

| Gap | Records |
|---|---|
| missing native `1::function_info::load_function_impl` | 57 |
| transaction shape: a framework without transaction_validation::versioned_prologue | 48 |
| unsupported: function values are not yet supported | 13 |
| missing native `1::string_utils::native_format` | 1 |

Error: 11 mismatch(es); 1 MonoMove crash(es)
```

**Exit code.** `aggregate` exits non-zero when any record mismatched or crashed, when no record
reached the output comparison, or when a record of one corpus appears in two inputs. That makes it
usable as a pass/fail gate.

**Outputs.**
- `--summary-out` writes the merged summary as JSON.
- `--markdown` appends the report to a file, so several runs can report into one.
- Both must differ from each other and from every `--results` input.
- The gap ranking is the list to work from when deciding which MonoMove features unblock the most
  real transactions.

---

## 8. Legacy dumps

Transactions collected earlier with `aptos-move/aptos-e2e-comparison-testing` can be converted into
a corpus.

### 8.1 Survey

`survey` shows what a dump holds before you import it:

```console
$ $B survey --legacy-dump mainnet-1-2m-new/
era:         Some(OnChainFramework)
versions:    Some(1404262) ..= Some(1531769)
records:     8329 indexed, 8232 in version_index.txt, 8329 state files (…)
decoded:     8329 ok, 0 failed
txn kinds:   {"user/entry_function": 8329}
packages:    1 distinct, top [("AptosFramework@1", 8329)]
key kinds:   {"resource": 83311, "table_item": 8329}
size:        1442.7 MB of values as dumped; as a corpus 5.1 MB pooled per 500-record shard + …
```

The `era` says which framework the dump ran against:
- `OnChainFramework`: the on-chain framework at each version.
- `CompiledFramework`: a framework compiled from the dump's `aptos-commons`, git `main` at dump
  time.

### 8.2 Import

```console
$ $B import --legacy-dump mainnet-1-2m-new/ --out corpus-legacy --corpus-id legacy-sample \
    --network https://archive.mainnet.aptoslabs.com --cache-dir import-cache --limit 50
wrote corpus "legacy-sample" to "corpus-legacy": 50 records in 1 shards, 70 modules, 1 frameworks
{
  "imported": 50,
  "skipped": {},
  "with_onchain_status": 50,
  "frameworks": 1,
  "fetched_modules": 0,
  "completed_values": 50,
  "completed_absent": 124,
  "without_features": 0,
  "v2_crashes": 0
}                                               # 27 s
```

A dump lacks the framework modules, the reads that found nothing, the on-chain status and the
auxiliary info. `import` fetches all of them from an archive node: public fullnodes prune old
history. It also always drops the dump's recorded feature flags, which the dump tool may have
overridden, and fetches the chain's own. That is why `completed_values` (or, for a chain with no
flags yet, `completed_absent`) is not 0 here.

**Options:**
- `--framework onchain` (the default) pairs each record with the framework on chain at its version.
  `--framework head` uses the framework this binary was built with.
- `--limit N` imports only the first N records.
- `--offline` needs no network. It implies `--framework head`. Records then lack their on-chain
  status and the chain's feature flags, so they compare as `incomplete`.

**Cache.** `--cache-dir` caches state reads, framework releases and full pages of committed
transactions, so an interrupted import reruns with little refetching. The cache is best-effort: an
entry that cannot be read or written is fetched again. It may not be inside `--out`, or `--out` inside it,
and it should be a directory only you write to: entries are written through whatever it holds. If an import is killed rather than
stopped, remove the half-written `--out` before rerunning.

### 8.3 What the comparison shows

```console
$ $B compare --corpus corpus-legacy --out compare-legacy.jsonl
{ "records": 50, "not_candidates": 2, "unsupported": 48,
  "unsupported_kinds": [ [ { "kind": "transaction_shape",
     "what": "a framework without transaction_validation::versioned_prologue" }, 48 ] ], … }
```

This is expected: MonoMove's executor needs `transaction_validation::versioned_prologue`, which
frameworks gained only in April 2026. Older records therefore cannot run on MonoMove against the
framework they ran with. Legacy dumps remain useful for checking V1 replay. For comparing MonoMove,
use fresh captures (§3).

---

## 9. Practical notes

**Time.** Measured on 2026-10-09 at the default `--concurrency 32`, with no API key:

| Step | Time |
|---|---|
| `targets` | seconds |
| `capture`, contiguous range | about 0.35 s per transaction |
| `capture`, targeted versions | about 0.8 s per transaction (more distinct modules and state each) |
| `import` | about 0.5 s per record |
| `compare` | about 0.1 s per record |

Most of capture's time is network round trips: V1's state reads, and fetching each record's module
closure. The framework releases are resolved a shard at a time, for all its transactions together
(normally two registry lookups), and each distinct release is fetched once per run.
`--complete` adds a replay of both VMs per record. Check
`wc -l versions.txt` before a large capture. To speed it up:
- `--api-key <KEY>` lifts the anonymous REST rate limit, which is usually the bottleneck.
- `--concurrency` sets how many transactions are captured at once. Without an API key, mainnet's
  public endpoint started returning `429 Too Many Requests` at 64, and the transactions whose reads
  failed were skipped (counted in the report as "capture failed"); 32 was safe.
- Page and release fetches are retried, but a transaction's own state reads are not: one that fails
  transiently is skipped as "capture failed". Capture such versions again on their own.

**MonoMove's limits.** Each MonoMove run is held to `--v2-memory-limit-mb` (default 4096) and
`--v2-timeout-secs` (default 120), on `capture`, `import`, `compare` and `replay`. Lower them to
make crashes fail faster. Several MonoMove runs go at once only while their reservations (each run's
limit plus an allowance for starting) fit in about three quarters of the machine's (or
container's) memory; on a small machine the memory limit is lowered to fit, and a run too large
for the budget still runs, alone. `--v2-memory-budget-mb` sets that budget instead.

**Partial corpora.** A `capture` or `import` that stops partway does not lose its work:
- The records are kept in `<out>.partial` (or `<out>.partial.<n>`), with a `STOPPED` file saying
  why, and `--out` stays free. If `--out` is the current directory, or the move fails, it stays
  in place, marked.
- `compare` works on a partial corpus and warns that it is one. In the rare case the corpus could
  not be marked, its records are kept without a manifest, which `compare` cannot open.
- A failure with a single record, such as a fetch that failed for one transaction, only skips that
  record and is counted in the report's `skipped`.

**Harness errors.** If the isolation harness itself fails, the command stops with an error and
blames nothing on MonoMove. That happens when the MonoMove child cannot start, speaks the wrong
protocol, or exits without a result. A common cause is rebuilding the binary mid-run; rerun it.

---

## 10. FAQ

**Why is `matches` (almost) always 0?** Nearly every record where the VMs agree still differs in at
least one known way. MonoMove does not emit state-value metadata yet, and it writes entries whose bytes did
not change. Both are documented TODOs in the executor. `known_difference` is the passing verdict
until they are fixed.

**Why is gas zero everywhere?** MonoMove has no gas meter yet. Both VMs run with gas zeroed, so
their outputs can be compared byte for byte. Gas parity is out of scope.

**Isn't V1 compared against the chain?** Only to select records. A record is compared only if V1
reproduces the chain's status on the patched state; otherwise it is `not_candidate`. (Offline
imports have no on-chain status; they are compared unchecked, and `unchecked` counts them.) After that,
MonoMove is compared against V1. Zeroed gas makes both replays differ from the chain in the same
way.

**What does `incomplete` mean, and what do I do about it?** A VM read a key the corpus has no
observation of: neither a value nor a recorded absence. The tool will not guess, so it decides
nothing for that record. `import` completes the state to avoid this; `capture` does with
`--complete`. If a fresh capture has `incomplete` records, recapture them with `--complete`.

**A record is `mono_unsupported`. Is that a bug?** No, it is a feature MonoMove has not
implemented yet, reported by MonoMove itself. `unsupported_kinds` ranks them. One exception: if
the gap appears only when writing the output, and MonoMove's status already differed from V1's,
the record is a `mismatch`. A gap never hides a divergence.

**Why does `replay` refuse some records?** A record whose transaction stopped at a metering limit on
chain (out of gas, say) is not replayed: with gas zeroed, a loop that gas stopped on chain could
run for ever. `compare` reports such a record as `not_comparable`, and `capture` and `import` skip
it. V2 is also not run when V1 stops at a metering limit the chain's status does not show (a
memory-limit stop before gas feature version `RELEASE_V1_38`): V1 runs first even with `--vm v2`,
unprinted. That stop is trusted only if V1 read nothing the corpus never observed; otherwise V2 runs,
under its isolation limits. `capture` skips those too.

**Can I trust a `known_difference`?** It means every difference is one of the two documented
kinds, checked difference by difference. Any other difference in the same record makes it a
`mismatch`. The design doc explains the exact rules.

**How do I compare after changing MonoMove?** Rebuild, then rerun `compare` on the same corpus.
No capture is needed.
