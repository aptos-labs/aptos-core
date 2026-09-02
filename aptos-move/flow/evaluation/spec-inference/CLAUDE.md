# Specification-Inference Evaluation

This directory contains a reproducible framework for evaluating Move-prover
specification inference. The intended study, its experimental arms, and
its reporting requirements are defined in the
[design](DESIGN.md).
[`DESIGN.md`](DESIGN.md) describes the architecture: what a task is, where the
corpus comes from, how a round executes, and how a result is scored. Read that
design, `DESIGN.md`, this file, `README.md`, and `corpus-v1/README.md` before
making material changes.

## Goal and current checkpoint

The study compares three deliberately different workflows using the same Move
task and model configuration:

- `agent_only`: direct AI specification inference; WP is unavailable.
- `hybrid_guided`: prescribed invariants → WP → repair → verify workflow.
- `hybrid_flexible`: WP is available, but the agent chooses its workflow.

The benchmark is an Aptos framework and experimental corpus, not a claim about
all Move code. It contains 30 tasks selected from a pinned Aptos revision (20
`aptos-framework`, 10 `aptos-experimental`). A full round has five fresh runs
per task and arm, for 450 sessions. Skills, prompts, tools, models, and limits
may be improved between rounds, but every change starts a new, recorded round;
do not overwrite or silently combine prior artifacts.

Do **not** launch a benchmark round until the shared dependency-contract gate
passes. The current authoritative gate is
`corpus-v1/metadata/dependency-contract-audit.json`; it currently has
`ready: true`. The latest audit has 307 ordinary opaque contracts, 12
documented expert assumptions, 28 private intrinsic-model boundaries, 40
native bindings, and 77 direct intrinsic bindings. It has no partial-abort,
untrusted-inference, incomplete-frame, or undocumented verification-disabled
entries. Regenerate the audit after contract changes instead of trusting stale
prose or counts in other documents. The current 30/30 compatibility screen
still predates dependency repairs and must be rerun before a benchmark round.

## Corpus model

`corpus-v1/framework/` is the single editable Move package shared by every
sample. It is a union of the selected targets' source-level dependency closure.
It is deliberately editable: dependency implementations may need loop
invariants or complete contracts to make the corpus provable.

`corpus-v1/samples/<task-id>/README.md` is the human-facing task recipe. It names
the target, target source file, dependency closure, aliases, allowed edits,
hashes, and preparation patch. Samples are overlays; do not create or maintain
independent framework copies for each one. The package's module-to-file mapping
is `corpus-v1/framework/corpus-modules.json`.

Keep durable corpus evidence inside `corpus-v1/`:

- `manifest.json`: source identity and all sample recipes.
- `metadata/`: inventory, selection, contract audit, target-body proof output,
  and recorded repairs.
- `screening/`: treatment-blind compatibility results.
- `patches/`: replayable preparation changes.

Only generated development-round material belongs in
`evaluation-artifacts/`. Do not leave phase-specific or one-off JSON files
beside the corpus root; either remove superseded generated artifacts when
explicitly authorized or consolidate stable metadata under `corpus-v1/metadata/`.

`AF` means **Aptos Framework** and `AX` means **Aptos Experimental**.

## Dependency and contract methodology

There are three distinct graphs. Do not substitute one for another:

1. The package/module closure is the transitive local dependency closure needed
   to compile a standalone shared package, including resolved named-address
   aliases.
2. The executable call graph determines which function contracts form an opaque
   proof boundary. Traversal stops at `pragma opaque`, native, or bodyless
   functions because callers see their contract rather than their body.
3. The specification graph follows ordinary spec functions and behavioral
   predicates (`result_of`, `ensures_of`, `aborts_of`, and related forms), even
   across modules. A caller's specification may invoke specification functions
   from a dependency, so those definitions must also be available and trusted.

For every authored opaque dependency contract:

- It must be complete before a caller consumes it. Select the function directly
  in the unchanged package and prove its body in callee-to-caller order.
  `pragma opaque` remains present: it controls how callers reason about the
  function and does not disable verification of the selected function's body.
- `pragma verify = false` is an explicit trusted boundary, equivalent to
  assuming the complete specification at that opaque call. It is permitted in
  the prepared dependency package only when an adjacent source comment says
  whether (a) the target-body proof succeeded at a recorded larger timeout and
  names its artifact, or (b) this is an expert assumption with a concrete
  rationale. A diagnostic body-proof attempt may use a temporary package copy
  that changes only `verify = false` to `verify = true`; leave `opaque` intact.
  Record the same classification in
  `corpus-v1/metadata/trusted-verification-boundaries.json`. Undocumented skips
  remain audit blockers. This exception never applies to specifications
  produced by an experimental arm: agents may not claim success by disabling
  verification.
- It needs precise normal-result and abort behavior. Never make a proof pass by
  deleting an `[inferred = sathard]`/`vacuous` clause, adding a weakening
  precondition, or setting `aborts_if_is_partial`. A trusted boundary does not
  make an incomplete contract complete.
- An opaque function that can mutate global state needs `modifies` targets for
  every resource type reachable through its executable closure. A mutable
  argument or a read-only global access alone does not require a global frame.
- `pragma intrinsic` is modeled by built-in prover semantics and needs neither
  an authored opaque contract nor a Move body proof. Native bindings use their
  declared boundary. Do not manufacture opaque specifications for either class
  solely to satisfy the audit.
- Private helpers that manipulate only the hidden representation of an
  intrinsic map can retain verification-disabled implementation contracts.
  Record them with basis `intrinsic_model`; the audit reports these separately
  from expert assumptions and never schedules a routine body proof for them.
- `spec_exists_at<T>(a)` must model `exists<T>(a)` exactly in specifications.
  The distinction between `exists` and `exists_at` is Move-code visibility; the
  prover has global specification visibility. The opaque Move helper
  `exists_at` can ensure equality to `spec_exists_at`, but the latter must not
  be made opaque if callers must prove resource creation.

When a compiler, WP, Flow, sourcifier, or prover defect is found, fix it and
add focused regression coverage where practical; it is not a candidate
exclusion. Re-screen all affected tasks afterward. Behavior-preserving repairs
to shared dependency code are allowed for a documented proof/infrastructure
reason. Preserve the rationale and proof evidence in
`corpus-v1/metadata/prover-repairs.md`.

## Loop, HOF, and `sathard` methodology

`[inferred = sathard]` is a repair obligation, never a disposable inference
artifact. It has more than one source:

- an uninvariant loop, whose havoc forces WP to quantify mutated state;
- a top-level hard quantifier; or
- an untrusted `result_of` carrier for an opaque or verification-disabled
  callee.

Do not turn every `sathard` case into a loop: first inspect the condition and
its source. The last category needs an exact callee value contract, not a loop
rewrite.

When the function is a private implementation helper of a type already modeled
with `pragma intrinsic`, classify that helper as intrinsic as well when its
result exposes only the hidden representation. Keep the complete public
intrinsic-map contract as the semantic boundary and record why in
`prover-repairs.md`; this is a model-boundary classification, not a weakened
opaque contract.

For a genuine loop case:

1. If it is an ordinary source loop, add a real `while ... spec` invariant:
   bounds, prefix/suffix facts, accumulator relation, and resource/frame facts
   as required by the intended contract.
2. If the loop arose through an inline higher-order iterator, first try the
   inline-HOF fold mechanism. A generic iterator can characterize accumulated
   captured state with `folds_of` and a suitable recursive `spec_fold` /
   `spec_fold_idx` declaration.
3. If fold derivation is unavailable (state-dependent iteration arguments,
   unsupported mutable lambda parameters, or a capture update that has no
   functional value model), rewrite the iterator as a source `while` loop and
   supply ordinary invariants.
4. Conversely, a simple accumulator loop can be expressed as an inline HOF if
   its behavior is naturally a fold and the `folds_of` invariant is derivable.

The prover now records loops without invariants in
`third_party/move/move-prover/bytecode-pipeline/src/loop_analysis.rs` and emits
a source-located diagnostic from `spec_inference.rs` when they lead to
`sathard` WP output. That diagnostic identifies inline-expanded loops and
states the fold-versus-explicit-loop alternatives. The inliner already reports
the precise reason when `folds_of` derivation cannot be exact. Do not broadly
mark all loop-derived contracts `sathard`: a loop whose invariants constrain
its havoc should not taint every inferred condition.

Current direct examples worth revisiting are `code::publish_package` and
`staking_contract::update_distribution_pool`, whose effects need genuine
invariants/functional helper contracts. The pure search loop in
`bulk_order_utils::discard_price_crossing_levels` now has prefix invariants.
`code::check_dependencies` has explicit-loop invariants after replacing an
effectful `vector::any`; re-run WP before judging its remaining clauses.
`code::publish_package` now has invariants for its deploy-owner, package-scan,
and reset loops. Its focused WP inference originally uncovered a
global-invariant diagnostic-dump panic; the dump must skip
`no_verified_bytecode` functions because the analysis intentionally leaves
them without annotations. The current compact registry-effect trial reaches a
60-second target-body proof timeout, and its complete forwarding contract is
still outstanding; do not classify it as trusted or remove its audit blocker.

### Current bottom-up `staking_contract` work

The path is now bottomed out. Read-only lookups, `pool_u64` share operations,
`stake::{withdraw_with_cap,unlock_with_cap}`, and reward arithmetic have compact
contracts with strict 60-second target-body evidence. The commission
helpers have exact state transformers and are documented expert assumptions
only where unified recursive abort coverage exceeds the solver budget.

`update_distribution_pool` uses an explicit indexed loop because the inline
HOF fold cannot represent its captured mutable `Pool`. Its abstraction is an
exact shares-map/shareholder-vector fold. `distribute_internal` adds an exact
first-shareholder redemption fold, operator-beneficiary redirection, dust
handling, final empty-pool state, and a finite payout frame. The finiteness
comes from the representation fact that the pool is populated only for the
staker and operator. `unlock_stake` composes distribution, commission, capped
principal reduction, distribution buy-in, and stake unlock through pure state
transformers. The two vesting wrappers forward those contracts without adding
state behavior.

These payout functions are explicit expert boundaries because their
target-body proofs would compose the already documented
`update_distribution_pool` and `aptos_account::deposit_coins` boundaries; they
are not partial contracts. The latter declares the complete account, APT
pairing/supply, and primary-store footprint. The audit understands that a
concrete `CoinInfo<AptosCoin>` frame covers the inventory's generic
`CoinInfo<#0>` callee effect while still comparing two concrete instantiations
exactly.

For the recursive commission fold, use Z3's native QI defaults. Split mode
suppresses explicit QI-threshold overrides because both tested forced settings
were worse. A higher eager threshold admits more costly quantifier instances;
change the global default only after a broader prover A/B regression.

`move-flow experiment prove --timeout N` treats `N` as a per-VC solver time.
Use `--split-vcs-by-assert` to retain per-assert diagnostic shards. Split mode
disables the aggregate package deadline because applying one VC budget to the
whole sequential shard set killed later assertions before they ran. A larger
target-body proof timeout is legitimate evidence; if the resulting complete
contract is made a trusted boundary for routine runs, record the successful
timeout and artifact beside `pragma verify = false`. An expert assumption must
be labeled as such. Neither form permits `aborts_if_is_partial` or a weaker
contract.

Behavioral predicates expose call-site contracts: `[concrete]` conditions are
proof-only and must not be included in `aborts_of`/`ensures_of`. The Boogie
translator now applies the same abstract/concrete and injected/export filters
as ordinary call-spec translation; the regression is
`closures/opaque_behavioral_predicates.move`.

Function-valued struct fields must be modeled whether or not their function
type has `store`. A direct forwarding wrapper such as
`market_types::validate_bulk_order_placement` needs exact `result_of<field>`
and `aborts_of<field>` conditions; it has no global frame. If its target-body
proof appears to treat the callback result as arbitrary, inspect
`mono_analysis.rs` and the Boogie `$apply` procedure before weakening the
contract. The regression case is
`move-prover/tests/sources/functional/closures/result_of_old_label.move`.

Intrinsic map constructors allocate a fresh hidden iterator-validity token for
each returned map object. Therefore a wrapper around `new_with_config` must not
assert equality to a separate `result_of<new_with_config>` call. State its
complete observable map abstraction instead: zero `spec_len` and no
`spec_contains_key`. The intrinsic binding supplies those facts; no opaque
contract is authored for the intrinsic function itself.

## Key implementation locations

- `harness/prepare.py`: builds the union package, aliases, sample recipes, and
  preparation patches.
- `harness/dependency_contracts.py`: computes contract status and frames.
- `harness/verify_dependency_contracts.py`: audit-gated, bottom-up proofs of
  ordinary opaque function bodies with opacity preserved.
- `harness/compatibility.py` and `harness/screen.py`: treatment-blind compile,
  WP, and prover screening; candidates exceeding the configured threshold are
  replaced only through the deterministic reserve hierarchy.
- `harness/controller.py`, `state_machine.py`, and `judge.py`: isolated model
  sessions, arm-blind follow-ups, and fresh-process scoring.
- `config/corpus.json` and `config/default.json`: selection and execution
  configuration.
- `../../cont/skills/move-inf/SKILL.md` and `../../cont/templates/`: Tera
  rendered skills. Keep shared reference material byte-identical across arms;
  vary only the purpose-built workflow and WP availability.
- `../../src/experiment.rs`: Flow's experiment commands and audit-facing
  treatment of untrusted inferred conditions.

## Safe working sequence

1. Read the sample README and target source plus its `.spec.move` file.
2. Use Flow/compiler query and package inventory to establish the relevant
   executable and specification dependencies; do not guess from filenames.
3. Repair one coherent dependency boundary or loop at a time. Preserve all
   existing user work in the dirty tree.
4. Prove a strengthened opaque dependency bottom-up before relying on it in a
   caller. Record the output in `corpus-v1/metadata/`.
5. Refresh the audit and then rerun compatibility screening for affected
   samples. A target that exceeds the configured screening threshold is
   excluded/replaced only before arm runs and only through the recorded reserve
   hierarchy.
6. Start pilot or full sessions only when the manifest/audit gate is genuinely
   ready. Never use an arm's outcome to choose corpus membership.

The user has explicitly authorized fixing Flow and prover bugs discovered by
this work. Do not change target behavior merely to simplify an evaluation
sample. Do not modify real framework packages outside this isolated corpus
without a separate request; the normal cached-package rebuild rule applies to
`aptos-move/framework/`, not to `corpus-v1/framework/`.

## Running and verification

Run commands from this directory unless a command says otherwise. The standard
commands and artifact paths are in `README.md`; the key dependency gate is:

```bash
move-inference-verify-dependency-contracts \
  --package corpus-v1/framework \
  --manifest corpus-v1/manifest.json \
  --trusted-boundaries corpus-v1/metadata/trusted-verification-boundaries.json \
  --move-flow /absolute/path/to/move-flow \
  --output corpus-v1/metadata/dependency-contract-verification.json
```

Real Claude Code + GLM development runs use the local wrapper through the
provided sandbox script; it maps the Z.ai credential to the Anthropic-compatible
endpoint without exposing the key:

```bash
sandbox/with-glm-env.sh .venv/bin/python -m harness.pilot_preflight ...
```

The wrapper pins the GLM aliases used by `/home/wrw/shared/bin/claude-alt glm`.
Never print, commit, or copy credentials into run artifacts.

The controller loads the selected generated plugin exactly as Flow is normally
used (`claude --plugin-dir <plugin>`) and starts the first turn with
`/move-inf`. Do not copy a rendered skill into the controller prompt or ask the
model to call an unavailable `Skill` tool; the plugin slash command is the
arm-specific workflow boundary.

For this ongoing work, run `cargo +nightly fmt --all` after Rust changes. The
user has asked to defer routine full lint/test sweeps until later; do not run
them automatically every turn. When verification is requested, start with the
smallest affected Flow/prover/corpus check and report exact artifacts and
timeouts.
