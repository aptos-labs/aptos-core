# Hosted Rust CI and AWS fallback

Rust lints and full unit tests use the prebuilt `64cpu-aptos-image-ubuntu22-x64`
runner by default. Targeted PR tests retain their existing AWS runner and selection
logic. No branch-protection settings or required lint check names change.

## Switching providers

Set repository variable `RUST_CI_PROVIDER=aws` to route subsequent lint/full-test
jobs to the original AWS paths. Set it to `hosted`, or remove it, to use hosted.
The Lint+Test manual-dispatch `rust_ci_provider` input overrides the variable for
that run only. Invalid provider values fail rather than silently selecting a cloud.
This PR does not change repository variables.

This is operator-controlled fallback, not automatic failover: already queued or
running jobs do not move, and hosted test failures are not retried on AWS. If the
image or runner pool is unavailable, select AWS and start a new run. A stale image
fails toolchain/setup hash verification; rebuild it or select AWS.

## Full-suite execution

The reusable workflow builds the nextest archive once, produces four compressed
runtime bundles, and runs deterministic hash partitions. Debug sections are removed
only from disposable runtime copies, not the original build outputs or Cargo build
flags. Stack traces from shard binaries consequently have less source detail.
VM-feature and doc tests run independently alongside the archive build. Package
exclusions, existing CI default filters, thread requirements and retries remain.

Each shard checks out the same workflow revision, restores helpers to original
build paths and populates offline dependency sources required by compile tests.
Shard caches are restore-only: saving the extracted/test-mutated target directory
caused ENOENT during Cargo cache cleanup in the trial and adds unnecessary writes.
The final verifier requires every phase to succeed, disjoint selected inventories,
exact union coverage, and matching completed test counts. Missing/empty results fail.

Four shards are the maximum per run, not a global concurrency limit. Multiple runs
can use the shared pool concurrently; existing parent concurrency cancels superseded
runs. Runner-pool/global capacity changes are outside this PR.

## Actions/runtime updates

Changed workflows pin checkout v7.0.1, upload-artifact v7.0.2 and download-artifact
v8.0.2 to their release commits. These use Node 24 rather than deprecated Node 20;
the artifact releases also include throttling/retry improvements. Shared unrelated
workflows are not upgraded here.

## Evidence

Verified trial: https://github.com/aptos-labs/aptos-core/actions/runs/38967167300

- Trial commit: `38ef306822` (based on application source `328ac1b3d4`).
- Image: `aptos-runner-image-ubuntu22-x64` 1.0.0; consumer currently selects latest.
- Full workflow: 29m34s, first job start to final verification completion.
- Same-source historical AWS full-suite job: 65m10s.
- Standard-image hosted sharding: 37m29s.
- Coverage: 14,942 tests across disjoint hash shards [3740, 3719, 3727, 3756].
- Comparison is one run per configuration, not repeated controlled averages.
- The application source on this PR is newer; live PR validation is required.
- September Azure billing marks Linux B64 Job credit-eligible. Avoided AWS cash
  savings depend on commitment coverage and residual storage/network costs.

The repository's scheduled pruner may delete runs of trial workflows absent from
main. Before deletion, full logs, API metadata, test inventories and shard results
were downloaded. A compact verified evidence snapshot is committed beside this
file; its data comes from that run rather than a synthetic benchmark.

## Local checks

```sh
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
actionlint .github/workflows/lint-test.yaml .github/workflows/hosted-rust-unit-tests.yaml .github/workflows/validate-hosted-rust-ci.yaml
git diff --check
```

Tests require PyYAML and zstd. Archive tests preserve helper files and reject invalid
paths; coverage tests reject missing, duplicate, failed or mismatched results.
