# CI validation plan

## Branch or PR?

A committed, pushed branch is sufficient for benchmarking and manually invoked
CI checks. Use the existing `Lint+Test` dispatch entry point with the branch ref;
pushes to ordinary feature branches do not automatically trigger this workflow.
GitHub CLI supports selecting the branch with
[`gh workflow run --ref`](https://cli.github.com/manual/gh_workflow_run).

A real PR is still needed before merging to validate PR-event behavior: affected
package selection, labels and skip conditions, required-check integration, and
PR-specific permissions. Branch dispatches do not establish those properties.
See [ci-performance.md](ci-performance.md) for the current implementation.

## Execution plan

1. **Freeze the candidate.** Commit and push the final worktree, record its SHA,
   and keep the branch unchanged during comparisons. Record the actual run SHA,
   workflow revisions, runner types, and image digests; do not mix evidence from
   different code revisions.
2. **Validate correctness.** Run local helper tests and manually dispatch regular
   `Lint+Test` plus the CLI dependency check. Confirm the expected lint, doctest,
   workspace-test, and eight smoke partitions execute rather than merely skip.
   Manual `Lint+Test` runs workspace tests instead of the PR-only targeted job.
3. **Measure targeted tests.** Run `small`, then `medium`, then `large`, waiting
   for each dispatch to finish. Each supplies three baseline/candidate samples
   with the same source, package selection, profiles, and retry settings. Require
   inventory equality and complete, disjoint partition coverage. Investigate
   failures or regressions before escalating to the next suite.
4. **Validate CLI/API and smoke gains.** Exercise all three CLI networks, both
   API spec comparisons, and smoke artifact transfer/resource usage. Fresh speed
   or compute claims need matched baseline runs, not candidate-only successes.
   Hold source, test inputs, and image digests fixed; vary only the workflow
   configuration being compared. Report runner-family differences explicitly.

Replace `YOUR_BRANCH` below. Run each command separately after the preceding run
finishes: `Lint+Test` runs on the same SHA share a cancellation group.

```bash
gh workflow run lint-test.yaml --ref YOUR_BRANCH -f ci-benchmark=false
gh workflow run cli-external-deps.yaml --ref YOUR_BRANCH
gh workflow run lint-test.yaml --ref YOUR_BRANCH -f ci-benchmark=true -f benchmark-suite=small
gh workflow run lint-test.yaml --ref YOUR_BRANCH -f ci-benchmark=true -f benchmark-suite=medium
gh workflow run lint-test.yaml --ref YOUR_BRANCH -f ci-benchmark=true -f benchmark-suite=large
```

**CLI/API comparison wiring is additional work.** These reusable workflows are
not directly dispatchable, and `docker-build-test.yaml` calls CLI E2E at `@main`.
Use a temporary, unshipped validation caller through an existing dispatchable
entry point to select the candidate and baseline workflows explicitly. Pin
workflow refs and use matching prebuilt images/test sources. Keep this harness
out of the shipping changes, preserve permission boundaries, and do not treat a
green `@main` run as candidate-workflow validation. The retained benchmark covers
targeted tests only; CLI/API and smoke A/B comparisons are not automated by it.

## Evidence and acceptance

Record fresh results here as a concise summary, with raw logs/metrics linked:

- Candidate and baseline definitions, run links, sample counts, and test coverage.
- Median/range of elapsed time and allocated vCPU-minutes; setup, build, transfer,
  and test phases; artifact sizes, peak memory, and slowest-shard time.
- All failures, retries, cancellations, skips, and rerun costs. Reused artifacts
  must not be counted as rebuilt; expired artifacts require rebuilding.
- A per-workload conclusion: improvement, neutral, regression, or inconclusive.

Use at least three matched samples for latency/compute claims. Require matching
coverage, passing candidate checks, and investigation of failures, OOMs, timeouts,
or material slowdowns; do not discard failed samples or relax tests to get green.
Keep scheduling and benchmark-only inventory overhead separate from execution.
Allocated vCPU-minutes are not billed-dollar savings, and per-job improvements
cannot be added into a whole-PR speedup. Preserve evidence before the one-day
build-artifact and seven-day metrics retention windows expire.

**Results:** Pending validation of the final committed revision. Manual results
can demonstrate gains and detect regressions, but do not replace PR-specific CI
or prove that every workload is regression-free.
