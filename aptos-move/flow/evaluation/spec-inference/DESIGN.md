# Evaluation Architecture

How the specification-inference evaluation is put together: what it measures,
what a task is, where the corpus comes from, how a round executes, and how a
result is scored.

This is the design of both the study and the apparatus that runs it. The
command-level runbook is [`README.md`](README.md); working rules for agents
editing this tree are in [`CLAUDE.md`](CLAUDE.md). No measured results appear
here; those live in per-round reports.

## 1. What is measured

One question: **does weakest-precondition inference help an AI agent write a
correct Move specification, and at what cost?**

Three arms run the same task, the same model, and the same configuration. Only
the workflow and the availability of WP differ:

| arm | WP available | workflow |
|---|---|---|
| `agent_only` | no | the agent specifies unaided |
| `hybrid_guided` | yes | prescribed: invariants → WP → repair → simplify → check |
| `hybrid_flexible` | yes | the agent chooses its own workflow |

`agent_only` has no simplification step because it is never given
mechanically generated conditions. In it the WP router is **absent**, not
discouraged: the tool cannot be listed or called.

Three contrasts are planned:

| contrast | meaning |
|---|---|
| `C1 = H-F − A` | the effect of making WP available in a goal-oriented workflow |
| `C2 = H-G − H-F` | the effect of prescribing the hybrid workflow rather than leaving it free |
| `C3 = H-G − A` | end-to-end comparison of the guided hybrid against direct AI |

`C3` is not a pure WP ablation, because capability *and* workflow both differ;
it is reported as an effect size with uncertainty rather than as a test. The
questions are whether WP improves success within a fixed budget, whether
guidance changes success or efficiency, and how the systems differ in token
cost, time, proof iterations, and specification quality.

The arm boundary is a **rendered Flow plugin**, not a prompt. Each round
generates one plugin per arm from the Tera sources in `../../cont/` into
`evaluation-artifacts/<round-id>/plugins/<level>/<arm>`, and records its
`plugin_manifest_sha256`; the controller launches `claude --plugin-dir <plugin>`
and opens with `/move-inf`. Rendering per round is what lets skills be improved
between rounds without a round ever mixing two versions. Shared reference material is byte-identical across
arms — only the purpose-built workflow section and the presence of
`move_package_wp` vary. That is what keeps the contrast attributable.

## 2. What a task is

A task is a **target function whose specification has been removed**, inside a
package that is otherwise complete and provable.

- `corpus-vN/package/` is a single editable Move package, vendored so it
  declares no external dependencies and relocates cleanly.
- `corpus-v1/samples/<task-id>/README.md` is the human-facing recipe: target,
  source file, dependency closure, aliases, allowed edits, hashes, preparation
  patch. Samples are **overlays**, never independent package copies.
- `materialize_task` copies the shared package, applies the preparation patch,
  and verifies `tree_hash` against the recipe's `expected_sha256`. It refuses to
  overwrite an existing tree.
- The agent's editable surface is `sources/**/*.move`. The target file starts
  with no specification for the target; dependency contracts under
  `sources/deps/` are present on purpose, as the trusted opaque boundary the
  agent reasons against.

Preparation is deliberately minimal: bodies are copied byte-for-byte, and only
two transformations are allowed, both recorded in the module header — reducing a
carrier struct to the fields the target reads, and turning a global config read
into a parameter.

## 3. Where the corpus comes from

### Lineage

| corpus | source | status |
|---|---|---|
| [`corpus-v1/`](corpus-v1/README.md) | Aptos framework + experimental, pinned revision | superseded as a benchmark; retained as infrastructure |
| [`corpus-v3/`](corpus-v3/README.md) | Etna | **the benchmark.** The full run is planned on this corpus |

**V3 is the final benchmark target.** Rounds are planned and reported against
it; V1 is kept for the reasons below, not as a run target.

*V1* was replaced because the Aptos framework is public and so are its
specifications — 16 of 24 checked targets already had a published upstream
specification for the very function the agent was asked to specify, so a success
may be recall rather than inference. It is kept because it is the only
**publishable** corpus, V3's sources being non-redistributable, and because it
is the only source of higher-order/iterator and global-state coverage, which the
V3 pool structurally lacks. Its 8,000-plus lines of authored opaque dependency
contracts are the proof infrastructure behind
[`corpus-v1/metadata/prover-repairs.md`](corpus-v1/metadata/prover-repairs.md).

*V2* was an earlier cut of the same Etna source and has been removed; it lives
in git history. It saturated — nearly every cell succeeded for every arm, so
most of a round carried no information — and it scored operational success only,
which a *vaguer* contract passes more easily. V3 answers both: targets selected
to resist guessing, and mutants so contract strength is measurable. Its source
selection reasoning survives in §3 and the appendix below.

The two corpora share no files. Each vendors its own dependency closure at its
own pinned revision: V1 carries the full framework closure with authored
contracts, while V3 vendors a trimmed standard-library slice so the package
declares no dependencies and relocates cleanly — a property the screen and the
run controller both rely on.

### Why a private source

The Aptos framework is public, and so are its `.spec.move` files and the
prover's own inference fixtures. A success on such a target may be recall rather
than inference. Etna — the codename for Decibel's private Move code — is not
public, so its functions and specifications are candidates for genuinely novel
tasks.

**No Etna source is in this repository.** `aptos-core` is public and Etna is
not, so everything derived from it is generated from a pinned commit of a
private repository — `aptos-labs/etna` at
`dd23678f980266360e050037fb78317b13753068` — into gitignored trees: the corpus
package (`corpus-v3/package/sources/`) and the reference packages
(`corpus-v3/references/build/`). What is committed is recipes, specifications,
digests, and anchors — our own text plus hashes. A mutant stores an offset and
a SHA-256 into the generated file rather than the code it rewrites, and a
reference is committed as a patch that only *adds* specification lines.
`build.py` exports the pinned commit rather than reading a working directory,
so the corpus cannot drift with someone's checkout. Reproducing the package
needs access to that repository; reading what the benchmark asserts does not.

The operative criterion is **specification absence, not code privacy.** A model
may well know public code, but it cannot recall a specification nobody wrote.
Measured over this repository, unspecified functions dominate everywhere —
`aptos-experimental` 213 of 213, `aptos-trading` 108 of 109, `move-stdlib` 208
of 289, `aptos-stdlib` 510 of 781, `aptos-framework` 1229 of 2024 — so the
criterion is not restrictive. It is what V1 failed: 16 of 24 checked V1 targets
had a published upstream specification for the very function the agent was asked
to specify.

Applying it, 19 of the 22 current targets are private Etna code, 2 are public
`aptos-experimental` (`extracted_bulk_order_utils`) that qualify because that
module carries no spec blocks upstream, and 1 is authored here — the only
function-valued target, which Etna could not supply.

Novelty is the point of the source survey; **dependency weight** is the
constraint that decides which candidates are usable.

### Novelty audit

Not all of Etna is novel. `move/aptos_market/` is a fork of the public
`aptos-experimental` order-book code — all sixteen files have a public
counterpart, several diverging by only tens of lines — and is **excluded
entirely**; it carries the same exposure the private source exists to escape.
The other eight packages (`perp`, `spot`, `accounts`, `vault`, `campaign`,
`trade_tracking`, `usdc`, `stablecoin_wrapper`) share no file name with
`aptos-move/framework` and form the pool.

### The dependency constraint

Import pressure across the private packages is dominated by exactly the things
that make opaque contracts expensive: `object::Object`, `fungible_asset`,
`big_ordered_map`, tables, coin, account, events, timestamps. Filtering modules
on those leaves almost nothing. Two observations recover the pool:

- **Module imports are not function dependencies.** A module with 49 `use`
  lines can contain functions that call nothing. Selection is per function.
- **Extraction beats mocking.** Where a good function sits in a heavy module,
  lift it into a minimal sample module rather than mock the module's
  dependencies. Mocking is needed only when the function itself touches
  framework state.

Three packages yielded nothing and should not be re-surveyed: `accounts`,
`usdc`, and `stablecoin_wrapper` are object/fungible-store orchestration end to
end, and `trade_tracking` is aggregators and tables over a clock. Note also that
`trade_tracking::unified_fees_config` duplicates the spot fee-tier logic — take
one side or the other, or the corpus gets two samples with one contract.

### Selection criteria

A candidate is admitted on four grounds:

1. **Cheap to prove.** Mutant scoring re-proves the target once per mutant, so a
   target near the timeout multiplies badly. Clean prove time must be
   comfortably fast.
2. **Resistant to guessing.** Targets small enough to hold in your head give the
   agent no reason to reach for a tool. Each task is labelled `hard` or
   `guessable`; a few `guessable` controls are kept deliberately, so an
   apparatus failure can be told apart from a genuinely difficult task.
3. **Discriminating rather than representative.** Paired targets of identical
   shape and opposite edge behaviour, and functions with aborts the source never
   mentions, probe exactness directly.
4. **Auxiliary reasoning the contract cannot state directly.** Some contracts
   are not provable from an invariant alone: they need a *spec function* to name
   the accumulated value and a *lemma* to relate it to itself — monotonicity of
   a running sum being the canonical case, where ruling out an intermediate
   overflow needs "every prefix is bounded by the whole", which is induction the
   solver does not do. Inventing that scaffolding is a distinct inference skill
   from writing invariants, and a corpus with no such target does not measure
   it. Prefer the cheapest target that still forces it: the reasoning should be
   the difficulty, not the solver time. In particular, prefer **linear**
   accumulation, since a nonlinear term multiplies the cost without adding to
   what is being tested.
5. **Stratum coverage** against the manifest's feature strata.

Two coverage gaps are known and stated plainly: the dependency-light pool
contains **no function-valued or inline-iterator material**, so that stratum
is covered by an authored target rather than an extracted one, and `folds_of`
coverage still has to come from the framework corpus; and the import-free tier
is global-state-free by construction, so resource frames and `modifies`
coverage come only from extracted config-reading targets.

### What the current corpus selects for

Composition the agent must reason through rather than pattern-match. The
flagship target calls two siblings, and its freedom from underflow holds only
because of a bound established in a *callee's* contract and invisible in its own
body — WP leaves that obligation open verbatim, and it cannot be discharged from
the caller alone.

The mirror-image target is abort-free only under an invariant established at
registration and nowhere visible in the function. A contract claiming
`aborts_if false` with no matching precondition is wrong, and looks right.

For loop targets, every arm is warned that an invariant is missing — WP will not
hand back a specification built on unconstrained state — and every arm that can
call WP also gets the bounded-unrolling evidence attached to that warning, which
exhibits the loop-head facts directly. The evidence is deliberately not gated by
feedback level: it explains *why* a loop is hard rather than supplying the
answer, so withholding it would make a diagnostic worse without making the task
harder in a way worth measuring. The arms therefore differ on whether WP is
available at all, not on how much it says.

## 4. How a round executes

A round is `tasks × arms × replicates` runs, each an isolated model session.

**Schedule.** `harness/schedule.py` builds randomized blocks seeded from
`selection_seed + round_id`. Blocks are shuffled, and **arm order is shuffled
within each block**, so position in the run sequence is not confounded with arm.

**Session.** `harness/controller.py` drives a multi-turn Claude Agent SDK
session under fixed limits (controller turns, model turns per controller turn,
wall seconds, output tokens). The opening turn is `/move-inf` followed by
`prompts/initial.txt`. That prompt is the same text for every arm and carries
only what the skill cannot know — which target, which package, and that a budget
bounds the session. Every normative instruction (preserve behavior, preserve
user-written specifications, finish with a full proof) belongs to the skill, so
the plugin remains the sole arm boundary and there is one source of truth per
rule. `harness/state_machine.py` supplies arm-blind follow-ups from
`harness/state_machine.py` supplies arm-blind follow-ups from
`prompts/followups.json`, keyed by outcome (compile failure, prover failure,
timeout, forbidden weakening, no progress). It never learns which arm it is
prompting. Built-in tools are
allowlisted — file tools yes; `Bash`, `WebSearch`, `WebFetch`, and subagent
spawning no — so the only route to the compiler and prover is Flow's MCP tools.

**Follow-up policy.** Nondeterministic answers make a fixed transcript
inappropriate, so every arm gets the same opening message and an arm-blind
controller picks a standard follow-up from fresh workspace and prover state:
retry a genuine infrastructure failure once from the identical snapshot; return
exact compiler diagnostics on a compile failure; return exact locations for a
forbidden weakening or out-of-scope edit; return prover diagnostics for a
logical failure; report the target and limit on a prover timeout; give a neutral
continue prompt when nothing relevant progressed; and stop on operational
success or the shared budget. It never reveals reference clauses, mutant
results, adequacy scores, or arm-specific hints.

**Isolation.** `harness/pilot_sandbox.py`, reached through
`scripts/pilot-sandbox`, runs each session under bubblewrap and Landlock as two
independent layers. The outer namespace has `--unshare-user/pid/ipc/uts`,
`--new-session`, `--uid 0 --gid 0`, `--cap-drop ALL`, a tmpfs `/tmp`, a fresh
`HOME` under the run's artifacts, and only the round's own inputs plus the
`claude`, `move-flow`, `boogie` and `z3` binaries bind-mounted. `/proc` is
mounted read-only because Boogie's self-contained CoreCLR reads `/proc/self`
at startup and aborts without it.

The inner Landlock ruleset then confines the agent process to less than the
sandbox holds: it writes its workspace and reads its plugin and `move-flow`,
but the harness, the prompts, the task patch, the pristine package and the
Boogie executable -- the apparatus it is being measured against -- stay out
of reach, and of `/proc` it gets only `/proc/self` and two read-only files.
Landlock resolves `/proc/self` once, to the wrapper's own PID, so no process
the agent spawns could start Boogie; the agent's `BOOGIE_EXE` is therefore a
client that hands each invocation to the controller over a run-local socket
(`harness/boogie_proxy.py`), and the controller runs the executable with the
working directory checked to lie inside the run. Preflight proves all of it
each time: an unmounted host file must not open, the agent must read its own
`/proc`, another process's must be refused, and `move-flow` must prove a
one-line package from a shell under the agent's ruleset -- Z3 probed by
`move-flow`, Boogie through the proxy, Z3 again under Boogie: the whole chain
the tools really run, which exec'ing one solver from the wrapper does not
exercise.

**Workspace.** Copied fresh from the baseline for every run and on every in-run
reset. Because `HOME` is a tmpfs, no `~/.claude` exists inside: no transcripts,
no memory directory, no history, no resumable session. Run *N* cannot observe
run *N−1*.

## 5. How a result is scored

Two levels, deliberately separate.

The prover budget is **40 seconds per verification condition, uniform across
tasks** — no per-task override. The reference specifications prove in 0.7–1.2s,
so that is ample for a correct contract, and a target that needs more is
exercising the timeout tactics the skill documents rather than being handed a
larger budget. The session's wall budget is set far above it so an agent can
afford several attempts. When a proof does exceed the budget, the check reports
`prover_timeout` and carries the solver's quantifier-instantiation and
nonlinear-arithmetic evidence, so the next attempt can be informed rather than
blind.

**Operational success** is awarded only when a fresh judge process confirms all
six of these within the operational proof limit: the package compiles; the
complete requested scope verifies; no verification skip or weakening construct
is present; runtime bytecode is unchanged; edits stay within the allowed scope;
and the required contract categories are present. `harness/judge.py` delegates
to `move-flow experiment check-candidate`, the same command the agent-visible
check runs, and the criteria are published outside the agent's workspace so they
cannot be relaxed by editing it. The agent's own success claim is never a score.

Forbidden shortcuts are `pragma verify = false`, partial abort contracts,
unconditional `aborts_if true`, vacuous result clauses, and invented
preconditions that exclude legal inputs.

Recorded alongside it: eventual verifier success at the larger limit; input,
cache-creation, cache-read, output and total tokens; cost recomputed from the
round's archived price table; end-to-end, model, Flow, compile, WP, prover,
hook, and judge time; tool and prover calls, counterexamples, timeouts,
controller turns and model turns; contract-category coverage and specification
size; and, for `hybrid_flexible`, WP adoption and action order — analysed by
intention to treat, since choosing not to call WP is part of that arm.

Operational success is necessary but weak: a *vaguer* contract passes it more
easily. Two arms can produce contracts of visibly different strength and score
identically.

**Mutation adequacy** — does the specification reject wrong code? A mutant is a
patch to the **implementation**, never to the specification. The harness applies
it to a copy of the package and re-runs the prover against the agent's finished
specification. Prover fails → **killed**, the contract was precise enough to
notice. Prover succeeds → **survived**, the contract verifies against wrong code.
`mutation_adequacy = killed / essential`, and `strict_success` requires
operational success *and* every essential mutant dead.

The current set is three mutants per task, one per obligation the contract must
pin, split across `normal-result` and `abort` categories. A mutant becomes
`essential` only once a hand-authored reference specification is shown to kill
it; mutants and references are authored **before** a round and **without seeing
any arm's output**, for the same reason corpus selection is treatment-blind.
Scoring runs after the round, because the agent shares the sandbox mount
namespace and hidden material must never be mounted alongside it.

Two properties make mutation the more trustworthy metric: mutants are code the
model has never seen, so the question is not answerable from memory; and the
metric rewards precision where verification rewards vagueness.

### Measuring cost

Local execution is not metered; inference is. The primary efficiency measures
are therefore inference API seconds, metered input/cache/output tokens, and
provider cost recomputed from the archived price table. Local Flow, WP,
compiler, prover, hook, and judge times are recorded as diagnostics, and
end-to-end wall time is a safety limit rather than a headline number — it is
also sensitive to prompt-cache warmth (§6).

One accounting trap is easy to fall into and hard to notice. The runtime mixes
per-turn and session-cumulative fields in a single record: `usage` and
`duration_ms` describe one turn, while `model_usage`, `total_cost_usd`, and
`duration_api_ms` are session totals that already include every earlier turn.
Summing the second group counts the same inference repeatedly, and the error
grows with turn count. **Take the last value per session and add across
infrastructure retries**, which start a fresh session. Inference time exceeding
a run's wall time is the signature of having got this wrong.

Turn count is a first-order cost driver twice over, since cache-read tokens
scale with turns and are billed. Reducing repair rounds pays in both output and
cache-read categories, which is why the arm workflows converge on a single
closing check rather than a verify/check alternation.

`harness/mine.py` labels archived transcripts by turn use, token category, and
failure kind; `harness/taxonomy.py` holds those labels against the diagnostic
categories the feedback design proposed, and reports which never fired and
whether any task could have produced them.

## 6. Repetitions and analysis

A full corpus round is five fresh runs for every task and arm. Runs are blocked
by `(task, replicate)`; the three arm orders are randomized within blocks and
all six orders are balanced across the schedule. Concurrency is fixed, and
members of one block do not contend on the same local solver lane. Failures
consume the shared budget and are retained rather than retried away.

**The task is the primary independent unit.** For each contrast, report
task-level success differences with 95% task-cluster bootstrap intervals, and
use a blocked randomization test as a secondary check. Control the family-wise
error rate across the planned contrasts `C1` and `C2`; report `C3` as an effect
size. Analyse time and tokens to success as restricted means under the common
cap, treating failures as censored at that cap. Mutation and
feature-stratified analyses are secondary.

Do **not** report a naive binomial interval over all runs: repetitions of one
task are not independent observations, and treating them as such overstates
precision by roughly the replicate count.

Skills, prompts, tools, limits, and scoring may improve between rounds. Record
each round's effective configuration and its relationship to earlier rounds,
retain the artifacts, and label results by configuration. Never silently pool
results from materially different configurations.

## 7. Validity controls

**Apparatus identity.** Every run records `config_sha256`,
`controller_harness_sha256` (a `tree_hash` over `harness/`),
`controller_prompts_sha256`, `plugin_manifest_sha256`, `move_flow_sha256`,
`mutant_manifest_sha256`, and `initial_tree_sha256`. Editing the harness
mid-round fails the identity check rather than silently mixing apparatus
versions — which is the intended behaviour, and why one-off analysis scripts
live in `analysis/`, outside `harness/`.

**Treatment blindness.** Corpus membership, screening, replacement, mutants, and
references are all decided without running an experimental arm. A sample the
screen did not clear carries a non-`ready` `screening_status` in the manifest,
and the scheduler drops it — naming it explicitly is an error rather than an
override, so an exclusion cannot decay into operator memory. A target that
exceeds the screening threshold is replaced only *before* arm runs, and only
through the deterministic reserve hierarchy.

**Round discipline.** Skills, prompts, tools, models, and limits may be improved
between rounds, but every change starts a new, recorded round. Prior artifacts
are never overwritten or silently combined.

**Vacuity.** A contract whose assumptions are contradictory verifies, passes the
check, and means nothing — and every outcome recorded for that run is then
meaningless, including mutation, which reports every mutant as surviving. It is
a scoring hazard, not a curiosity: two sources have been found this way and
fixed. `move-flow experiment prove --check-inconsistency` detects it, and
`harness.validate_mutants` rejects a reference the prover reports inconsistent.

**Contamination** has its own section (§8), since three different things get
called by that name and only one of them is live.

## 8. Contamination

A recurring question is whether a run can be tainted by something cached from an
earlier run. Three distinct mechanisms travel under that name. They have
different answers, and conflating them makes the apparatus look either safer or
more compromised than it is.

### Provider prompt cache — not a correctness threat

Roughly 95% of input tokens in a round are cache reads. That is expected and is
almost entirely *within-session* prefix re-reading, which every agentic turn
does: each turn resends the conversation so far.

It cannot taint an outcome. A KV prefix cache is keyed on the exact token
prefix and returns the activations a cold computation would have produced, so it
is semantically transparent. There is no mechanism by which the *content* of a
different session enters this one — a cache hit reproduces this session's own
prefix, not another's continuation.

It does affect **wall-clock**, because a warm prefix is served faster, and runs
of the same arm share a long identical system-prompt-plus-skill prefix. Two
consequences, both already handled: wall time is a diagnostic and a safety
limit rather than a headline number (§5), and `schedule.py` shuffles arm order
within each block, so cache warmth is spread across arms instead of being
confounded with one.

### State carry-over between runs — closed by construction

This is the mechanism that *would* taint results, and it is the one the sandbox
is built to exclude.

- `/home` is a **tmpfs** and `HOME=/home/eval` is a fresh directory per
  invocation. There is no `~/.claude` inside: no transcripts, no memory
  directory, no history, no resumable session. `/tmp` is tmpfs too.
- The workspace is copied fresh from the baseline for every run *and* on every
  in-run reset.
- `materialize_task` refuses to overwrite an existing tree and verifies
  `tree_hash` against the recipe's `expected_sha256`; every run records
  `initial_tree_sha256`.

So run *N* cannot observe run *N−1*'s specifications, build directory, or notes.
The sandbox preflight re-proves the isolation on each launch (§4).

A related question is whether the answer is simply *present* in the workspace.
It is not: the target file starts with zero specification lines for the target,
and the only `.spec.move` files in the tree are dependency contracts under
`sources/deps/`, which are there deliberately as the trusted opaque boundary.
The task's own reference specification and its mutants are never mounted — the
agent shares the sandbox mount namespace, which is why mutation scoring runs
after the round rather than beside it.

### Pretraining memorization — real, mitigated, not eliminated

The model may have seen a target and its specification during training. This is
the irreducible one, and the reason the benchmark moved to a private source.

What remains after that move is bounded and stated: 8 of 10 targets are private,
and the 2 public ones carry no upstream specification (§3). Three further
arguments limit the damage:

1. **The comparison is within-subject.** Same task, same model, same
   configuration; only workflow and WP availability differ. Memorization
   inflates all three arms alike, so the between-arm contrast — the claim the
   study makes — survives it. What it would threaten is an absolute claim about
   AI specification ability, which this design does not make.
2. **Mutation adequacy is largely immune.** A mutant is code the model has never
   seen, so "does this contract reject wrong code" cannot be answered from
   memory. That is a further reason to treat mutation, not operational success,
   as the headline metric.
3. **Obfuscation was considered and rejected.** Renaming addresses none of the
   above: it does nothing for the first two mechanisms, and against the third it
   is weak — a model recognizes structure, not identifiers, and the targets are
   already extracted into corpus-specific modules under new names. Its cost is
   real: it changes every tree hash, invalidates the contract audit, breaks the
   preparation patches, and forces a re-screen.

**Open.** A closed-book probe would measure the residue rather than argue about
it: a fresh session, no tools, shown only the module header and target
signature, asked to produce the specification. Reproducing the reference marks
that target as memorized, giving a per-task contamination score to report
alongside the results. It costs one cheap session per target and has not been
run.

## 9. Artifacts

Durable corpus evidence lives under the corpus tree: `manifest.json` (source
identity and sample recipes), `metadata/` (inventory, selection, contract audit,
target-body proof output, recorded repairs), `screening/` (treatment-blind
compatibility results), `patches/` (replayable preparation), and `mutants/`.

Generated round material lives under `evaluation-artifacts/<round-id>/`, one
directory per run holding the baseline and final trees, the workspace diff,
the Claude and controller event logs, the candidate check, the judge decisions,
and the mutation score.

### Reproducibility record

Every run archives its round, task, arm, replicate, block order and limits;
every implementation, plugin, prompt, skill, model, tool-inventory and source
hash; Claude events, Flow telemetry, controller decisions, stdout/stderr, and
both UTC and monotonic timing; the initial tree identity, the final diff and
files, and the fresh-judge result; and the raw model identifier with token
categories, without credentials.

The round manifest additionally records the source commit, the Flow / Claude /
SDK / Boogie / Z3 versions, the sandbox identity, the randomization seed and
schedule, the scoring version, and the contemporaneous price table. Raw token
counts are the stable resource result; dollar costs are derived from them.

### Go/no-go before a benchmark round

- [ ] Every scheduled sample's `screening_status` is `ready`, and the screen is
      current with the package identity the round will run.
- [ ] Each module has a committed reference specification, and every essential
      mutant is validated against it with its digest recorded.
- [ ] `agent_only` cannot list, call, or reach WP; the hybrid tool inventories
      match each other.
- [ ] Opening prompt, non-WP capabilities, limits, and arm-blind follow-ups are
      identical across arms.
- [ ] Tactic, plugin, runtime tool inventory, skill hash, and run manifest agree.
- [ ] Every run starts from the recorded task hash in a fresh session and
      workspace.
- [ ] No verification skip, partial abort contract, runtime edit, or hidden-file
      access can count as success.
- [ ] Model identity, raw token categories, timings, failures, retries, and
      termination reasons are complete.
- [ ] References and mutants stay inaccessible to the agent and never guide
      repair.
- [ ] Analysis uses task-level blocking and includes failed runs at the cap.

**The Etna sources are not committed.** `aptos-core` is public and Etna is not,
so `corpus-v3/package/sources/` is gitignored and only the recipe lives here;
`corpus-v3/build.py --verify` regenerates in place and fails, naming files, if
any digest differs from the manifest. Any published artifact built from this
corpus needs its own disclosure decision — contract shapes can be described
without reproducing proprietary source, but the package itself cannot be
redistributed without one.

## Appendix: the Etna candidate pool

From the source survey. Nothing here has a public counterpart. Targets already
promoted into a corpus are marked *(in corpus)*.

**Tier A — usable with no preparation.** Zero or near-zero imports, no global
state, self-contained aborts.

| target | shape and probe |
|---|---|
| `blended_oracle_util::calculate_weighted_price` | two-vector accumulation loop; three named aborts plus cast overflow |
| `price_management::get_median_price` | nested comparisons; total function, no aborts |
| `work_unit_utils::get_max_order_placement_limit` | clamped floor division |
| `work_unit_utils::consume_order_match_work_units` | saturating state transition through `&mut`; overflow abort |
| `liquidation_config::get_liquidation_margin` | two-branch ceil ratio; overflow and zero-divisor aborts |
| `liquidation_config::get_liquidation_price` | ceil ratio with compound divisor; an easily-missed zero-leverage abort |
| `adl_tracker::get_bucket_index` | bounded linear scan; least index satisfying a predicate |
| `trading_fees_manager::find_min_value` / `find_max_value` | identical loops, opposite empty-vector behaviour — an exactness probe as a pair |
| `trading_fees_manager::calculate_min_net_taker_fee` | chained floor percentages; subtle underflow abort |
| `fee_distribution::add` | enum `match`; partial monoid with four mismatch aborts |
| `oracle::calculate_deviation_bps` | abs-diff ratio with a `MAX_U64` sentinel instead of an abort |
| `payout_math::compute` | three-branch piecewise-linear interpolation over `u128` |
| `protected_trial::trial_size_for` | four-way `u128` product, truncating divide |
| `user_credits::credits_for_duration_days` *(in corpus)* | last-match-wins scan over a struct-invariant table |
| `spot_clearinghouse::compute_base_needed` | canonical accumulator invariant with overflow abort |
| `spot_work_unit_utils::get_max_match_limit` | clamped division; total |
| `spot_fees_config::compute_fee_from_basis_points` | exact ratio **plus two aborts the code does not guard** |
| `funded_first_trade::checked_max_tier_leverage` | universally-quantified precondition, max postcondition |
| `vault::convert_existing_shares_to_asset_amount` *(in corpus)* | pro-rata `mul_div`; zero-divisor edge |

**Tier B — usable after a one-line `#[verify_only]` wrapper**, the package's own
convention for specifying `inline` functions:
`work_unit_utils::consume_work_units`,
`price_management::apply_funding_rate_multiplier`,
`perp_market_config::safe_round_to_granularity`, and
`liquidation::min_liquidation_units` — the richest contract found, with a
closed-form `u256` ceil division, degenerate-denominator fallback, lot round-up,
min-size floor and clip.

**Tier C — needs extraction or mocking**, the preparation being to pass the
config reads as parameters: `position_update::is_settle_price_inside_guaranteed_range`,
`perp_market_config::round_size_to_lot`,
`slippage_math::compute_limit_price_with_slippage`,
`backstop_liquidator_profit_tracker::calculate_pnl` *(in corpus)*,
`open_interest_tracker::get_max_open_interest_delta_for_market`,
`spot_clearinghouse::compute_quote_needed` *(in corpus)* — a per-level floor-sum
that is deliberately *not* floor-of-sum, so bulk totals reconcile with per-order
escrow — `spot_engine::validate_order_input` *(in corpus)*, and
`spot_market_config::register_market`, which has nine argument-derived aborts
across three error codes and two `move_to`s.

**The vault share-math cluster** *(in corpus)* is the one place offering
*composed* contracts without framework weight: a high-water-mark fee feeding a
share-minting ratio feeding a redemption split, with a conservation property
across the split, using only `mul_div` and same-module predicates over `&Vault`.

**Existing private reference specifications** use `pragma aborts_if_is_strict`,
so they are exact rather than partial. They serve as scoring references, not as
inference targets: `i64_math.spec.move` (signed construction, `mul_div`,
`ceil_mul_div`, min/max, sign decomposition, with explicit boundary aborts),
`math.spec.move` (a `MulDivSpec` schema parameterised over ceiling versus floor,
a `NonZero` schema, and a `Precision` struct invariant), and
`perp_positions.spec.move`. The remaining `.spec.move` files in `perp` are stubs
with no `spec fun` bodies, so their functions remain available as targets whose
*dependencies* are already specified — exactly the condition a clean opaque
boundary needs.

**Practical constraints on any new Etna sample.** Dependencies are unresolved
and unpinned — no `Move.lock`, no `build/`, and `Move.toml` names a moving
branch — so samples must be repointed at the pinned local
`aptos-move/framework` to be reproducible. `decibel_dex` and `aptos_market`
both resolve to `0x4e110`. Most candidates are `package fun`, which verifies
fine but means an extracted sample keeps its module identity.
