# Bytecode verifier soundness

A Lean 4 specification of Move bytecode and machine-checked proofs that the
Move bytecode verifier (`third_party/move/move-bytecode-verifier`) guarantees
it. The plan is
[`../doc/designs/bytecode-verifier-soundness.md`](../doc/designs/bytecode-verifier-soundness.md).
[`SOUNDNESS.md`](SOUNDNESS.md) summarizes for reviewers the method, the
theorems and every assumption they rest on. This package depends on no other
Lean package in the tree.

## Status

**Phase 0 (foundations): done**, except sign-off of the trusted spec by human
clean-room reviewers.

- Syntax of `CompiledModule`, `CompiledScript`, and every instruction.
- Reference semantics, with the complete outcome classification (the R1–R7
  failure catalogue, rollback) and the module lock R4.
- Property statements and the theorems' exact shapes
  (`Spec/Properties/Theorems.lean`), over a fixed `VerifierInterface`.
- The production configuration recorded and checked against
  `aptos_prod_verifier_config` (A3).
- An independent clean-room review of `Spec/` by an agent that had not read
  the verifier; its findings are applied (see below).

**Phase 1 (core code): done.** **Phase 2a (data by value): done**, except one
clause of L6 (below), and pending sign-off of the new trusted statements.

The supported subset is now the core (integers of every width, bool, address,
locals, branches, non-generic calls, aborts, constants) plus data by value:
signers, vectors, and non-generic structs and enums, built and taken apart by
`Pack`, `Unpack`, their variant forms, `VecPack` and `VecUnpack`, and
`AbortMsg`. Abilities are checked on values, deeply: a value has the
abilities of its type narrowed by everything inside it.

- The model (`Model/`) transcribes every pass of `verify_module_with_config`
  except the verifier meter: bounds, features, limits, duplication,
  `signature_v2`, instruction consistency, constants, friends, recursive
  struct definitions, instantiation loops, and the code units (the struct-API
  checks of version 10, control flow with Tarjan's reducibility test, stack
  usage, type safety, locals safety, reference safety over the borrow graph,
  and the acquires list), plus the link checks. It agrees with the Rust
  verifier on every module recorded, accepting and rejecting alike
  (`Tests/ModelAgreement.lean`: `diff_core`, `diff_verifier`, the 29 modules
  of `diff_passes`, `diff_data`, `diff_databad` and `diff_constants`).
- N4 on the Aptos framework (`scripts/check-framework.sh`,
  `Tests/Framework.lean`): the model accepts all 191 modules of the release
  bundle (`head.mrb`), by local verification and link checks, as the Rust
  verifier does. The framework's `0x1::table` keeps the table rules. 178
  modules are in the supported subset, and 121 are covered together with
  everything they depend on, so the theorems apply to them; what keeps the
  rest out is 38 natives with references (aggregators, storage slots,
  dispatch, cryptography and others).
- Differential runs: the semantics agrees with the legacy Move VM and MonoVM
  on every call of `core.spec.json` (57 calls) and `data.spec.json` (16
  calls: structs, enums, vectors, the variant check R1, the vector length
  check R2, abort messages).
- Constant decoding (A4): on the 36 constants of `constants.spec.json`
  (every width, signers, trailing and missing bytes, non-canonical,
  truncated, oversized and multi-byte vector lengths, nested vectors), the
  production verifier, the model and the specification's decoder agree
  (`checkConstants`, `Tests/Differential.lean`).
- Proved, all from the trusted statements in `Spec/Properties/Theorems.lean`
  and with only Lean's standard axioms (`Tests/Axioms.lean`):
  - **D1** for the model, on the Phase 2 subset:
    `Proofs.soundness_byValue : SoundnessByValue Model.verifier`. A program
    the model accepts that uses no reference or global storage instruction
    or type (`Program.ByValue`) never goes wrong, whatever its natives do
    within their contracts.
  - **D1** on the whole stage-A subset, generic code included:
    `Proofs.soundness : Soundness Model.verifier`. A program the model
    accepts, on the supported subset, never goes wrong, whatever its natives
    do within their contracts; calls of struct-API mutable borrows, generic
    or not, run their wrappers while the caller's analysis takes them for
    field borrows of the argument. The Phase 3 result is the milestone
    `Proofs.soundness_nonGeneric` (`Program.SupportedNonGeneric`,
    `Proofs/NonGeneric.lean`).
  - **C1–C2** on the whole stage-A subset:
    `Proofs.certificatesHold : CertificatesHold Model.verifier`, with the
    type certificate expressed over each function's type parameters.
  - **D1 with references, milestone M3**:
    `Proofs.soundness_M3 : SoundnessM3 Model.verifier`. A program the model
    accepts, on the supported subset, whose function handles are no
    struct-API mutable borrows (`Program.M3`), never goes wrong: references
    are created, read, written through, frozen, compared and dropped, passed
    to callees and returned to callers, and borrowed from global storage; an
    execution may start with references to values the transaction holds, as
    entry functions take `&signer`; and `Exists`, `MoveFrom` and `MoveTo`
    use global storage. Milestones M2 (no global borrows) and M1 (function
    handles take and return values) are corollaries (`Proofs.soundness_M2`,
    `Proofs.soundness_M1`). Every module of the reference corpus that the Rust
    verifier accepts satisfies M3, and they pass references to calls and
    borrow global storage (`Tests/Milestones.lean`).
  - **C1–C2**: statically consistent on the whole Phase 3 subset
    (`Proofs.certificatesConsistent`); executions agree with them for
    programs by value (`Proofs.certificatesHold_byValue`).
  - **K1–K2**, K1 for function and struct handles, generic code included:
    `Proofs.linkedProperties : LinkedProperties Model.verifier`. A function
    handle's types equal its definition's at every instance, because the link
    check compares tokens structurally and struct handles naming one type
    have one arity (`compareTypes_ofTokenWith`, `sameArity`).
  - **L1–L11**:
    `Proofs.localProperties_of_phantom : PhantomUnderReferences → LocalProperties Model.verifier`.
    L7 and L11 are Phase 4's (below).
    Every property follows from the model's acceptance except one clause of
    L6, the phantom-position rule for field types that mention a reference,
    which is a hypothesis (`Proofs/Declarations.lean`). L3's reducibility is
    the largest part: Tarjan's check implies that every back edge of the loop
    summary's search enters through a dominator (`Proofs/LoopNesting.lean`),
    the search is a depth-first search whose finishing order decreases along
    its other edges (`Proofs/LoopSummary.lean`), and the two together rank
    the forward edges of the instruction graph (`Proofs/Reducible.lean`).
    L5 is `Proofs/TypeFormation.lean`; L6's other clauses and L10 are
    `Proofs/Declarations.lean`.
  - **N1** and **N2** (`Proofs.entriesAdmissible`).
- How D1 is proved:
  - `Proofs/Certified.lean` proves D1 from *any* consistent type and
    availability certificates plus side conditions (`Certified`): resolution
    of calls, successors, constants, declared abilities justified by the
    fields (`AbilitiesSound`), and `drop` for every local that `StLoc` or
    `Ret` may discard. It is stated purely over the spec, so it applies to
    any consumer of such certificates, MonoVM included.
  - The other files show that the model's own analyses provide them:
    - C1 from type safety (`Proofs/Typing.lean`). The model checks abilities
      against a module's own struct handles and the semantics against the
      definitions; K1 makes the former at most the latter
      (`abilities_sub`).
    - C2 and the `drop` facts from locals safety
      (`Proofs/Availability.lean`);
    - the abstract interpreter's scheduler reaches a post-fixpoint
      (`Proofs/Scheduler.lean`), using the depth-first traversal's
      properties (`Proofs/Traversal.lean`);
    - resolution of calls and struct types, and declared abilities, from
      duplication, the signature checker and the link checks
      (`Proofs/Linking.lean`);
    - successors, nonempty code and typed locals
      (`Proofs/Acceptance.lean`);
    - constants (`Proofs/Constants.lean`).
- Not modeled: the verifier meter, an acceptance limit, so the model accepts
  functions too costly for Rust to analyze; `control_flow_v5` (bytecode
  versions up to 5 are rejected). Type safety models the supported subset and
  rejects other instructions; reference safety, acquires and the struct-API
  checks are transcribed in full, but only their supported paths are
  exercised until type safety covers more (Phases 2b and 3).

Every instruction outside the supported subset is `wrong (unsupported _)`;
the `Supported` premise of every theorem rules it out until a later phase
covers it.

**Phase 2b (accounting and encapsulation): D2 and D3 proved**, pending
sign-off of the new trusted statements. The statements of D2
(`OwnershipAccounting`) and D3 (`Encapsulation`), the ghost layer they are
stated over, the entry premises E1 and E2, the natives' ownership contracts,
and N2 for transactions and N3, are in `Spec/` (below, "Awaiting sign-off").

- **D2**: `Proofs.ownershipAccounting : OwnershipAccounting Model.verifier`.
- **D3**: `Proofs.encapsulation : Encapsulation Model.verifier`.
- Stage B of Phase 3 extends the ghost layer to references and global
  storage; the proofs cover it.
- Neither uses the verifier's acceptance: both follow from the semantics, the
  native contracts and E1/E2. For a verified program, D1 excludes `wrong`, so
  every step either fails (and rolls back) or is accounted.
- How they are proved:
  - Every step that continues has an `Effect` on the state's occurrences:
    some are kept (possibly elsewhere), some discarded (with `drop`), some
    consumed by its events, some copied (with `copy`), and the rest created
    with fresh identities (`Proofs/GhostEffect.lean`). D2a, D2b, provenance
    and conservation follow generically from an effect.
  - Every core instruction that continues is annotated, and its annotation
    has such an effect and respects D3 (`exec_annot`, `coreStep_annot`,
    `Proofs/GhostExec.lean`, from the per-shape lemmas of
    `Proofs/GhostSteps.lean` and the helpers' shapes in
    `Proofs/GhostSemantics.lean`).
  - Every native call that returns is annotated under the native contract
    (`nativeAnnotated`, `Proofs/ModeledAnnotated.lean`; natives on values in
    `Proofs/GhostNative.lean`, the modeled natives with references in
    `Proofs/ModeledOwnership.lean`). The counting is over positions of the
    nodes the native sees: each passed-on node is kept once and copied when
    passed on again (only with `copy`), the others are consumed; what a
    reference argument designates and the native does not write back stays
    where it is, and a node of it passed on is a copy.
  - A transaction's initial shadow creates every node once, by the
    transaction, and E1 (all the way down) and E2 make those creations
    respect D3 (`entry_inv`, `Proofs/GhostInitial.lean`).
  - The trace invariant (`TraceInv`, `Proofs/Accounting.lean`) carries over
    every annotated step; D2 and D3 follow from it (`Proofs/Ownership.lean`).

**Phase 3, stage A (references and global storage in the semantics): in
`Spec/`, awaiting sign-off.** `Supported` now includes the non-generic
borrows, `ReadRef`, `WriteRef`, `FreezeRef`, `TestVariant`, the vector
operations through references, and the global storage operations, with the
checks R3 and R5.

**Phase 3, stage B (ghost layer, D2–D4, `&signer` entries): in `Spec/`,
awaiting sign-off.** The ghost layer annotates every instruction of the
subset, with global storage and the values the transaction holds as
occurrences; D2 and D3 cover storage, transfers through references and the
privileged operations; D4a–e are stated (`Spec/Properties/References.lean`,
theorem shape `ReferenceSafety`); entry functions take `signer` or
`&signer`, over any well-typed initial storage.

**Phase 3 proofs: in progress.**

- The model's type safety covers every instruction of the subset, and the
  model agrees with the Rust verifier on the 18 modules of `diff_refs`
  (`Tests/differential/refs/`), accepting and rejecting alike.
- C1 and C2 are statically consistent on the whole subset, references and
  global storage included (`Proofs.certificatesConsistent`); L5, L6 (with
  the variant-fields-shared clause now proved, not vacuous) and L9 cover the
  new instructions.
- D1 and the C1/C2 agreement are proved for programs by value
  (`soundness_byValue`, `certificatesHold_byValue`).
- D1 with references is proved for milestone M3 (`soundness_M3`): no
  function handle is a struct-API mutable borrow; references cross calls in
  both directions, entries take `&signer`, and code borrows global storage.
  M2 (`soundness_M2`, no global borrows) and M1 (`soundness_M1`, functions
  take and return values) are corollaries.
- D1 is proved on the whole supported subset (`soundness`,
  `Proofs/SoundnessAll.lean`). The analysis takes a call of a struct-API
  mutable borrow (`BorrowFieldMutable(off)`) for a mutable borrow of field
  `off` of the reference argument, while the semantics runs the callee. The
  callee is a wrapper whose code is `MoveLoc(0); <borrow of field off>;
  Ret` (`apiCall_wrapper`, `Proofs/StructApiFacts.lean`: the attribute
  reaches the definition through the link checks, and the struct-API checker
  checked its code, in modules of version 10 and later, the only ones that
  can carry the attribute). Executions keep the reference invariant or a
  wrapper in progress over a caller that called it from a state with the
  reference invariant (`ApiRun`, `InvAll`); the wrapper changes only its own
  frame, and its return resumes the caller as from a field borrow
  (`apiRun_call`, `apiRun_step`, `apiRun_ret`).
- How M3 is proved:
  - the abstract interpreter's post-fixpoint for any analysis and order,
    with an invariant of the recorded states (`JoinSound`, `JClosed`,
    `Proofs/Scheduler.lean`), and a property of every recorded pre-state
    from its closure (`analyze_justified`, `Proofs/Traversal.lean`);
  - the borrow graph, semantically: what each operation does to its edges,
    up to covering, and what the verifier's queries report
    (`Proofs/BorrowGraph.lean`);
  - the concretization of the reference-safety analysis
    (`Proofs/RefConc.lean`): edges true of the references they connect,
    chains of true edges, and completeness: a mutable reference with another
    strictly inside it has a chain to it, so the verifier's writability
    check means what it must. Completeness is kept by every operation that
    creates or consumes a reference.
  - the analysis's recorded states form a post-fixpoint for the order
    `RefLe` (an abstract state describes every frame the other describes),
    and each is well typed (`Typed`) against the type certificate's stacks
    (`Proofs/RefCert.lean`, `Proofs/RefModel.lean`);
  - the invariant of executions with references (`RefInvariant`,
    `Proofs/RefInv.lean`): the current frame agrees with the analysis's
    state before its instruction, its reference parameters standing for the
    references it was passed; every reference the analysis tracks is valid,
    of its type, and rooted in a local of its frame or an older one, a value
    the transaction holds, or a global resource (`Proofs/RefRuntime.lean`);
    and each suspended caller agrees with the analysis's state at its call
    (`CallFacts`). The references a caller keeps are shielded from the
    mutable references it passed, which are inside its own locals, its own
    global borrows or mutable parameters, so what its callees invalidate is
    what it allowed (`AllowedBy`);
  - global borrows across calls: the analysis names a frame's global
    borrows only for types its function acquires, while its module is
    active once (`gsidOf`); the active counts are the frames counted as
    activations (`ActiveCounts`); a frame protects the types it and the
    frames below borrowed (`GlobOk`), which the frames above acquire only in
    a module active again, where R3 makes global operations fail. This rests
    on the acquires verifier (a same-module callee acquires only what its
    caller does) and on the analysis's call, which rejects a callee
    acquiring a type the caller borrowed (`Proofs/RefAcquires.lean`,
    `Proofs/RefGlobalFacts.lean`);
  - a write through a tracked reference keeps the description of every frame
    it may reach (`Proofs/RefUpdate.lean`);
  - a call passes references that are exclusive where mutable, the analysis
    having checked that the mutable ones are writable; the callee's checks at
    its return make the references it returns inside its parameters and
    compatible with each other (`return_contract`), and the caller resumes
    with them as the analysis's results of the call (`resume_frameOk`,
    `Proofs/RefCallComplete.lean`, `Proofs/RefCalls.lean`);
  - every instruction keeps the invariant (`Proofs/RefLocals.lean`,
    `Proofs/RefRefs.lean`, `Proofs/RefWrites.lean`, `Proofs/RefGlobals.lean`,
    `Proofs/RefGlobalFacts.lean`, `Proofs/RefCalls.lean`), assembled in
    `Proofs/RefSoundness.lean`.
- D2 and D3 are proved over stage B's ghost layer (`ownershipAccounting`,
  `encapsulation`).
- D4a follows from D4b′, D4c and D4e for any program and any natives
  (`noDanglingFromContainment`, `Proofs/Dangling.lean`). A reference is
  valid when created; a step keeps a valid location valid unless one of its
  events hits it: a direct write overlapping it, a write through a reference
  strictly enclosing it, or the end of its frame. A hit between a
  reference's creation and a later use is what D4b′, D4c and D4e exclude.
- D4b′ is proved for M3 programs (`mutableContained_M3`,
  `Proofs/MutableContained.lean`). How:
  - it reduces to a fact about one step (`mutableContained_of_steps`,
    `Proofs/Containment.lean`): a reference used later is held at every step
    until then, for any notion of held that a step's uses imply and that a
    step only narrows, apart from what it borrows (`HeldFacts`);
  - the references the analysis tracks are such a notion: those on operand
    stacks and in the locals the availability analysis has available
    (`heldFacts_tracked`, `Proofs/RefTrackedFacts.lean`). A reference left
    in a local available on one path into a join only is released by the
    analysis, and cannot be used again;
  - no access reaches over a tracked `&mut` of the current frame, by the
    analysis's checks (`contained_top`), nor of a suspended frame
    (`contained_below`): the frames above access, through references, only
    their own locals, locations inside the references their callers lent
    them, and global storage no frame below protects. A suspended caller's
    references are tracked at its call (`CallFacts.tracked`), and none of
    its `&mut` references is strictly inside an argument, mutable or not.
- D4a, D4c and D4e are proved for M3 programs (`noDanglingUse_M3`,
  `Proofs/DanglingM3.lean`; `immutableStable_M3`, `deadSlotsUnborrowed_M3`,
  `Proofs/ReferencesM3.lean`), by the same reduction to one step over the
  tracked references.
- D4d is proved for M3 programs (`noEscape_M3`,
  `Proofs/ReturnsDerived.lean`). How:
  - references occur only as whole slot values, by the types (`unnested_M3`,
    `unnestedAfter_M3`, `Proofs/NoEscapeM3.lean`);
  - every reference a frame holds has its origin in the frame (`Origin`,
    `IdChain`): it derives from one of the frame's reference parameters, or
    from a borrow the frame made of one of its locals, or of a resource of a
    type its function acquires while the type's module is active once (R3).
    Steps keep this. A step adds references only to the current frame, the
    ones it borrows (`traced_frameRefsAt`, `Proofs/FrameRefs.lean`), and
    keeps each frame's identity, function and activation (`traced_shape`,
    `Proofs/FrameSteps.lean`). A callee's parameters are references its
    caller holds, and a callee returns references derived from them;
  - a verified `Ret` returns no reference into the frame's locals, nor into
    a resource of a type its function acquires unless the type's module is
    active twice (`ret_roots`, `Proofs/ReturnsM3.lean`): the analysis found
    each returned reference inside a parameter, and a parameter into a
    resource makes the frame protect its type. So a returned reference
    derives from a parameter.
- D4b″ is proved for M3 programs (`parametersExclusive_M3`,
  `Proofs/ReferenceSafetyM3.lean`). How:
  - it reduces to one step (`parametersExclusive_of_steps`,
    `Proofs/Exclusive.lean`): an access not through a reference derived
    from a `&mut` parameter `r` of a frame that is still on the stack overlaps
    no held reference derived from `r` (`StepsExclusive`);
  - each frame keeps facts about its parameters from its call (`ParamChain`,
    `Proofs/ParamFacts.lean`, from `call_suspend`). They are in no local of
    the frame or a newer one, and pairwise compatible. The global types they
    are in are protected: a frame at or above acquires one only in a module
    active twice;
  - an access through a reference of the current frame derives from another
    parameter of `r`'s frame, compatible with `r`. Otherwise it derives from
    a root borrow of a frame at or above it: a local, or a resource R3
    excludes (`idChain_down`). A direct access is to a local of the current
    frame, or of a resource that R3 or its absence excludes
    (`stepsExclusive_M3`, `Proofs/ExclusiveM3.lean`).
- D4, reference safety, is proved for M3 programs (`referenceSafety_M3`):
  accounting, D4a, D4b′, D4b″, D4c, D4d and D4e together.
- D4 is proved on the whole stage-A subset, generic code included
  (`referenceSafety : ReferenceSafety Model.verifier`,
  `Proofs/ReferenceSafetyAll.lean`; the Phase 3 result
  `referenceSafety_nonGeneric` is a corollary): each
  part's one-step form holds in a state with the reference invariant as for
  M3, and in a state with a struct-API wrapper in progress, whose steps
  access only the wrapper's own local, inspect and borrow through its
  argument, and return a reference derived from it (`Proofs/HeldAll.lean`,
  `Proofs/StepsAll.lean`, `Proofs/EscapeAll.lean`).

**Phase 4 (generics and function values): in progress.**

- The model's type safety covers the generic instruction forms
  (`typeCallGeneric`, `typePackGeneric`, `borrowFieldGeneric`, ...: the
  instantiation's type arguments substituted into the named entity's
  signature, as Rust's `instantiate` and `materialize_type` do). The model
  now models every instruction. The model agrees with the Rust
  verifier on the 13 modules of `diff_generics`
  (`Tests/differential/generics/`): every generic form, and type mismatches
  that show only after substitution.
- L4, L5, L6 and L9 cover the generic forms: their stack effects are their
  arities (`typeInstr_effect`), their instantiations are well formed with
  constraints satisfied (`functionArgs_wf`, `structArgs_wf`), a generic
  variant field borrow names a field its variants share at the instance
  (`variantFieldSharedAt_of_generic`, through `instantiateIn_spec`, the
  model's substitution agreeing with the specification's), and a generic
  form names a generic entity with one type argument per type parameter.
- The trusted statements of stage A (below) are merged. The theorems proved
  before them are milestones on the non-generic subset
  (`Program.SupportedNonGeneric`, `Proofs/NonGeneric.lean`): D1 and D4
  (`soundness_nonGeneric`, `referenceSafety_nonGeneric`) and the C1/C2
  results. K1–K2 hold for generic code (`linkedProperties`). A frame's function
  instance satisfies its constraints (`ActiveAt.typeArgs`), as the type
  certificate's consistency requires.
- D2 and D3 hold on the whole stage-A subset, generic code included
  (`ownershipAccounting : OwnershipAccounting Model.verifier`,
  `encapsulation : Encapsulation Model.verifier`): the ghost layer annotates
  every generic form as its non-generic form, and the accounting proofs cover
  them.
- **L7** (`Proofs/InstantiationsFinite.lean`): the model's instantiation-loop
  check, which rejects a type-constructor edge inside a cycle of the graph of
  (function, type parameter) nodes, implies that finitely many instances are
  reachable through a module's own calls (`instantiationsFinite_of`). A node's
  rank, the number of type-constructor edges it is reachable from, bounds the
  nesting of the types at it: the type arguments at a node of rank r lie in
  the r-th of a tower of finite sets of types.
- **L11** (`Proofs/WithinLimits.lean`, `Proofs/LoopDepth.lean`): the limits
  pass bounds the counts, fields, variants, parameters and returns, and the
  weighted size and depth of type tokens, whose weights are at least one
  (`withinLimits_of`); the code-unit verifier bounds the basic blocks; and
  the depth Tarjan's check computes for each collapsed loop bounds the number
  of natural loops that contain an instruction (`loopDepthAtMost_of`).
- With L7 and L11, `localProperties_of_phantom` proves `LocalProperties`,
  L1–L11, in its stage-A form (`Proofs/LocalAll.lean`).
- Instances (`Proofs/Instances.lean`, `Proofs/InstanceAbilities.lean`): a
  token instantiated by the analysis and translated under the caller's type
  arguments is the original token translated under the translated
  instantiation (`ofTokenWith_instantiate`); a token L5 calls well formed,
  translated under type arguments that satisfy the constraints in scope, is
  a type of the program with at least its handles' abilities
  (`translate_wf`); and the abilities the model computes are at most the
  runtime type's (`abilities_sub_gen`), given conservative struct handles
  (K1).
- C1–C2 hold statically for generic code: the model's certificate gives each
  instance the analysis's types translated under the instance's type
  arguments (`modelTypes`, `modelTypes_parametric`), consistent for every
  instance whose type arguments satisfy its constraints, every generic form
  included (`modelTypes_consistent_gen`, `certificatesConsistent_gen`,
  `Proofs/TypingGen.lean`, `Proofs/CertificatesGen.lean`).
- Declared abilities are justified at every instance (`abilitiesSound_gen`,
  `Proofs/AbilitiesGen.lean`): L6 checks a struct's fields with every type
  parameter assumed to have every ability, and the phantom-position rule and
  K1's conservative handles carry that check to any instance.
- D1 and C1 hold for programs by value with generics
  (`soundness_byValue_gen`, `certificatesHold_byValue_gen`,
  `Proofs/SoundnessGen.lean`): a generic call checks the type arguments the
  caller supplies against the definition's constraints, which the signature
  checker and K1 guarantee, and each generic form steps as its non-generic
  form at the instance (`execGeneric_safe`).
- D1 and C1 with references hold for generic code (`soundness`,
  `certificatesHold`, `Proofs/SoundnessD1.lean`). Executions keep the type
  and availability invariant together with the reference invariant or a
  struct-API wrapper in progress, generic or not (`InvAllG`, `ApiRunG`,
  `Proofs/ApiGeneric.lean`). The generic borrows and global operations reuse
  the non-generic proofs, now stated for any instruction and its type
  arguments (`Proofs/RefRefsGen.lean`, `Proofs/RefGlobalsGen.lean`). The
  analysis labels a global borrow by struct definition, so two instances of
  one generic resource share a label; the invariant compares resource types
  without their type arguments (`StructIdOf.erase`).
- D4 holds for generic code (`referenceSafety`). Its one-step forms are
  stated over a common view of the two wrapper runs (`ApiCore`, `InvCore`,
  `Proofs/ApiCore.lean`), and each generic form behaves as its non-generic
  form at the instance.
- **Stage A is proved**: D1–D4, C1–C2, K1–K2 and L1–L11 in the shapes of
  `Spec/Properties/Theorems.lean`, for the stage-A subset.
- **Stage B (function values)**: its trusted statements are merged (below).
  Every theorem above holds against them, in the shapes of
  `Spec/Properties/Theorems.lean`, and so does the new **D5**
  (`functionValueSafety : FunctionValueSafety Model.verifier`,
  `Proofs/FunctionValues.lean`): captured values hold no references, every
  closure is well formed with abilities within its target's and its
  captures', and every closure in storage targets a storable function.
  - Conformance now allows function-type subsumption. Two references whose
    targets differ only in a function type's abilities compare equal
    (`RuntimeType.comparable`, a spec fix found by the proofs). A transaction
    may lend a held closure at a type with fewer abilities; every reference
    to a whole held value has the lent reference's target
    (`RefInvariant.heldTargets`).
  - D3c, closures are unforgeable, is proved over the ghost layer; the
    ghost layer's accounting covers `PackClosure` and `CallClosure`.
  - The model's type safety now covers the closure instructions
    (`typeClosPack`, `typeCallClosure`, transcribing `clos_pack` and
    `call_closure` with `ClosureMask::extract`), and agrees with the Rust
    verifier on the 7 modules of `diff_closures`
    (`Tests/differential/closures/`). Every theorem above covers closure
    code: `PackClosure` and `PackClosureGeneric` are typed as the model
    types them (the closure's abilities: the target's, `store` only through
    a `persistent` handle, intersected with the captured values'), and
    `CallClosure` enters its target like a call whose target is known only
    at run time (`execClosure_safe`, `refRuns_callClosure`,
    `closure_suspendAt`).
  - A closure call has no static acquires list, so the caller's global
    borrows are protected by R3: closure dispatch counts the target's module
    active even within one module, so a global operation of that module
    fails with `RUNTIME_DISPATCH_ERROR` while a suspended frame holds a
    global borrow (`CountedOk`, `closure_suspendAt`).
  - Two spec corrections found by the proofs: a closure's abilities come from
    its captured values, not its captured parameters' declared types
    (`Program.closureValueAbilities`); and C1's certified successor types are
    supertypes of an instruction's result, since a closure's static type
    depends on the instance (`TypeCertificate.Consistent`).
- **Phase 4 is proved**: D1–D5, C1–C2, K1–K2 and L1–L11 in the shapes of
  `Spec/Properties/Theorems.lean`, for every program the model accepts on
  the supported subset, generic code, function values and calls of the
  modeled natives, the table natives among them, included (`soundness`,
  `certificatesHold`, `ownershipAccounting`, `encapsulation`,
  `referenceSafety`, `functionValueSafety`, `linkedProperties`,
  `localProperties_of_phantom`).
  N2 for transactions and N3 (`Spec/Properties/NonVacuity.lean`), open from
  earlier phases, are proved too, for every program
  (`transactionEntriesAdmissible` in `Proofs/NonVacuity.lean`;
  `nativeContractsSatisfiable` in `Proofs/NativeWitness.lean`, with the witness
  the statement describes). L6's phantom rule under references awaits the
  verifier fix.

**Phase 4, stage A (generics): in `Spec/`, awaiting sign-off.** `Supported`
now includes generic functions, structs and enums, type parameters in every
signature, generic natives that take and return values, and the generic form
of every instruction but `PackClosureGeneric`. A frame runs a function
instance with closed type arguments, struct types are instances, and every
generic form behaves as its non-generic form at the instance its
instantiation names, with the new wrong reason `constraintViolation`. L7
(local instantiation) and L11 (syntactic bounds) are stated and added to
`LocalProperties`; K1 compares type-parameter counts and constraints and the
types at every instance; C1's type certificate is expressed over each
function's type parameters. Function types and closures are stage B. The
proofs cover these definitions (above).

**Phase 4, stage B (function values): in `Spec/`, awaiting sign-off.**
`Supported` now includes function types in every signature and `PackClosure`,
`PackClosureGeneric` and `CallClosure`. A closure value records its function
instance, mask, captured values and type; its abilities are its target's
(`copy`, `drop`, and `store` for a storable target) intersected with its
captured parameters'. Conformance has the subsumption rule for function types
(`RuntimeType.assignable`). Closure dispatch counts the target's module as
active even within one module (R3), and the module lock applies to it (R4).
The ghost layer annotates the three instructions (`CallClosure` consumes the
closure, D2), D3 gains D3c (closures are created only where their target is
visible), K2 covers closure creation and the storability of `persistent`
handles, L7 follows closure creation, and D5 is stated
(`Spec/Properties/FunctionValues.lean`, theorem shape `FunctionValueSafety`).
Closures over natives, and natives whose signatures name a function type, are
outside the subset. The proofs cover these definitions (above).

**Natives with references (2026-11-03): in `Spec/`, awaiting sign-off.**
`Supported` now admits calls of the modeled natives of the standard library
(`Spec/Semantics/ModeledNatives.lean`, the one list to review), under the
language-owner rulings of 2026-11-03 (below). The natives are relations that see
what their reference arguments designate, under three shared shapes: read
(`vector::length`, the `string` internals, `bcs`, `cmp::compare`), update
(`vector::push_back`, `pop_back`, `swap`, `move_range`, `mem::swap`) and borrow
(`vector::borrow`, `borrow_mut`, `signer::borrow_address`). The
`#[bytecode_instruction]` vector natives are not defined as their instructions
(ruling of 2026-11-03). The semantics, the ghost layer, the native
contracts (`NativesRespect`, `NativesRespectOwnership`), N3 and the tests cover
them; every other native with a reference or a function type in its signature
stays outside. The language owners settled five of the six questions this step
raised on 2026-11-04, confirming the spec as written (below, "Open"); whether
N3 should ask the update and borrow shapes to return stays open. The proofs
cover these definitions (above).

**Table natives (2026-11-04): in `Spec/`, awaiting sign-off; the proofs cover
them.** Under the language-owner rulings of 2026-11-04 (below), the
six natives of `aptos_std::table` with references are modeled natives: read
(`contains_box`, `destroy_empty_box`), update (`add_box`, `remove_box`) and
borrow (`borrow_box`, `borrow_box_mut`). A table holds its entries
(`Value.table`): they are part of the value for ownership, abilities and
locations, and a reference to an entry designates a place inside the table
(`PathStep.tableEntry`). The natives are modeled only in a program whose
`0x1::table` keeps the table rules, checked on its bytecode
(`CompiledModule.followsTableRules`, the four checks of ruling 2 and three
refining it, rules 5 to 7), as part of their identification, so no theorem
gains a premise. `drop_unchecked_box`, a native on values, consumes a
table with its entries, as the native policy allows (`NativePolicy.destroys`).
The semantics, the ghost layer, the native contracts, N3 (unchanged as stated)
and the tests cover them.
The proofs cover these definitions (above). The table rules enter them in one
place: the ghost layer's `Unpack` needs an unpacked table to have no entries,
which rule 2 gives (`unpack_noEntries`, `Proofs/TableRules.lean`).

### Clean-room review (Phase 0)

The review found that `Soundness` could be stated over any predicate, that
module ids were not required to be distinct (a call could resolve into
another module's private function), that a caller's handle was never
compared with the callee's definition, that K1 held vacuously outside the
subset, and that C1/C2 did not make the certificates static. All are fixed;
`Tests/Review.lean` holds the counterexamples.

The review also found that the Move book said `%` aborts only on a zero
divisor, while the spec and both VMs fail on `MIN % -1`. The maintainers
confirmed the spec (2026-10-02); the book's correction lands separately, on
branch `move-book-signed-arithmetic`.

Two later changes to `Spec/` came from the proofs (Phase 1), both signed off
by the maintainers (2026-10-02):

- `SignatureToken` equality was a derived `BEq`, which Lean implements for a
  nested inductive as an opaque `partial` function, so no proof could use a
  token comparison. It is now structural recursion with a kernel-checked
  lawfulness proof, the same relation as Rust's derived `PartialEq`.
- `LinkedProperties` gained the `P.Supported` premise the other theorems
  have: K1 compares types as runtime types, which exist only on the subset.

### Awaiting sign-off

New trusted statements, written clean-room by agents that read only
`Spec/`, the plan, the `file_format.rs` doc comments and the Move book.

Phase 1:

- L8, `Spec/Properties/Constants.lean`: every constant has a primitive or
  vector type (no `signer`), and decodes to a conforming value.
- L9, `Spec/Properties/Instructions.lean`: generic instruction forms name
  generic entities with one type argument per parameter, non-generic forms
  name non-generic ones, `pack`/`unpack` name declared structs, and vector
  and closure-call signatures hold one type.
- N2, `Spec/Properties/NonVacuity.lean`.

Phase 2a:

- The semantics of data by value (`Spec/Semantics/`): struct types named by
  defining module and name and resolved by name, deep conformance of values
  to the declarations, deep ability checks, BCS decoding of vector constants,
  the new failures (R1 `variantMismatch`, R2 `vectorLength`, abort messages)
  and wrong reasons (`badOperand`, `unresolvedStruct`).
- K1 for struct handles (`Program.StructHandleResolution`,
  `Spec/Properties/Linking.lean`): every struct handle is a conservative view
  of its definition (`StructHandle.ConservativeFor`). Triage corrected it
  from equality: link checking accepts the weaker relation, which is sound
  because every ability check requires presence, never absence.
- L5, L6 and L10 (`Spec/Properties/Declarations.lean`). Triage corrected
  L6's `VariantFieldsShared`: it first required every variant field handle
  in the table to give its variants' fields one type token, which a module
  the language admits falsifies (a generic enum whose variants declare the
  field as a type parameter and as a closed type, borrowed where the two
  agree). It now requires each borrow to name a field its variants share at
  the instance it borrows.
- N2 premised on constructible parameter types (`Program.Constructible`):
  with structs, some types have no values.
- `Spec/Properties/Theorems.lean`: `LocalProperties` includes L5, L6 and
  L10, and `LinkedProperties` includes K1 for struct handles.

Phase 2b:

- The ghost layer (`Spec/Semantics/Trace.lean`): a shadow of the state that
  gives every node of every value an identity, annotates each step with the
  identities it creates and consumes (`Shadow.coreStep`,
  `Shadow.nativeStep`), and the annotated executions `Trace`. It changes no
  value, state or outcome: `step` stays the only source of outcomes, so D1
  and every earlier statement are unchanged. Copies keep identities; `Pack`,
  `PackVariant`, `VecPack`, loads, operator results, natives and the
  transaction create them; `Unpack`, `UnpackVariant`, `VecUnpack` and
  natives consume them. A step whose shadow does not fit is unaccounted, and
  D2 and D3 require every step to be accounted.
- E1 and E2 (`Spec/Properties/Entry.lean`): `EntryInitial`, a call of an
  entry function with no return types (added 2026-11-03) whose leading
  signers are for authenticated addresses
  (`auth`) and whose other arguments are of transaction-argument types,
  including the struct types the transaction layer accepts (`A`, each with
  `copy`, without `key`, with argument-type fields). It implies
  `AdmissibleInitial` (`EntryInitial.admissible`, proved in place).
- Permissions and the natives' ownership contract
  (`Spec/Properties/Natives.lean`): who may create, consume and take apart
  struct values and create signers; explicit native exceptions
  (`NativePolicy`); natives' identity flows (`NativeFlows`) and
  `NativesRespectOwnership`.
- D2a–c (`Spec/Properties/Ownership.lean`) and D3a–b
  (`Spec/Properties/Encapsulation.lean`), N2 for transactions and N3
  (`Spec/Properties/NonVacuity.lean`), and the theorem shapes
  `OwnershipAccounting`, `Encapsulation` and `TransactionEntriesAdmissible`
  (`Spec/Properties/Theorems.lean`).

Phase 3, stage A:

- References (`Spec/Semantics/Values.lean`): reference types `&T` and
  `&mut T` to value types, only as parameter, return, local and stack types
  (`RuntimeType.ofSlotToken`); a reference value is a location (a local of a
  frame instance or a global, and a path of field, variant-field and element
  steps), a mutability flag, its target type, and the number of
  invalidations recorded when it was created. References have `copy` and
  `drop`; a `&mut T` value does not conform to `&T` (bytecode freezes
  explicitly).
- State (`Spec/Semantics/State.lean`): frame identities never reused, global
  storage, and the invalidations that end locations' validity: moving a
  local or a resource out (every location in it, for good), a frame
  returning, and changing a value in place (`WriteRef`, overwriting `StLoc`,
  vector push, pop and swap: every location strictly inside the old value).
  Accessing an invalid location is `wrong` (`invalidReference`); copying,
  moving, freezing, passing, returning or dropping a reference is not an
  access. `returned` carries the committed storage; failures and limits
  roll it back (`RunResult.committed`).
- The instructions (`Spec/Semantics/Step.lean`) with the new wrong reasons
  `invalidReference`, `immutableReference` and `globalWithoutKey`, and the
  failure `emptyVector` (with R1, R3, R5 and R6 for the new instructions).
- `Supported`, `AdmissibleInitial` over well-typed initial storage
  (`GlobalStorage.wellTyped`), natives restricted to values, and C1's
  `typeAfter` for the new instructions.

Phase 3, stage B:

- The ghost layer (`Spec/Semantics/Trace.lean`) annotates every instruction
  of the subset. Global storage and the values the transaction holds are
  occurrences; a value in the initial storage was created before the
  transaction (`Creator.storage`). Reading through a reference copies the
  location's identities, writing moves the value in and discards the old
  one, and vector updates, `MoveTo` and `MoveFrom` are transfers. References
  have identities: borrows and `FreezeRef` create them (`borrowed`, with the
  reference they derive from); moving and copying keep them. Steps record
  uses of references, reads, writes and inspections of locations, locals
  becoming unavailable, frames entered and returning, and the operations at
  a struct type that create nothing (`privileged`, for D3a).
- D2c counts identities in the committed global storage; D3a covers field and
  variant field borrows, `TestVariant` and the global storage operations.
- D4a–e (`Spec/Properties/References.lean`) as executable trace definitions:
  dangling uses, conflicting accesses while a `&mut` is live, writes while a
  `&` is live, references inside values or storage and returned references
  not derived from a parameter, and locals or frames ending while a
  reference into them is live. Liveness keeps a parent's borrow alive on its
  child's location, so the field borrows compiled code makes through
  separate borrows of one local do not conflict. `ReferenceSafety`
  (`Spec/Properties/Theorems.lean`) for every admissible initial state.
- `&signer` entry parameters: the transaction holds the signers it lends
  (`Root.held`, `State.held`); `initialState` admits reference arguments to
  distinct held values, all well formed (`heldArguments`). `EntryInitial` takes `signer` or
  `&signer` leading parameters, no return types (2026-11-03), and any
  well-typed initial storage; N2 for transactions likewise.
- `Program.SupportedVersions` (`Spec/Properties/Safety.lean`) also states
  the deserializer's version-gated rule for function attributes: below
  version 10, no function handle carries a struct-API attribute
  (`FunctionAttribute.isStructApi`, `Spec/Syntax.lean`; the production
  deserializer rejects them as `MALFORMED`, `deserialize_function_attribute`
  in `move-binary-format/src/deserializer.rs`). The spec has no
  deserializer, so without it the theorems would cover modules that cannot
  come from bytes: an older module whose function carries a
  `BorrowFieldMutable` attribute the struct-API checker never inspects
  (added 2026-10-30, for D1 and D4 on struct-API borrows).

Phase 4, stage A (generics):

- Runtime types (`Spec/Semantics/Values.lean`): a struct type is a
  definition's name with closed type arguments (`StructId`, now
  `StructIdOf RuntimeType` with `typeArgs`, so that runtime types nest inside
  it); type parameters never occur at run time. `RuntimeType` has a
  hand-written `DecidableEq`, as `SignatureToken` and `Value` do. A function
  instance is a `FunctionRef` with `typeArgs`: frames run instances, and a
  native is called with one, so it receives its type arguments
  (`call_native(func.name, ty_args, args)`, `file_format.rs`). A non-generic
  entity has none, so non-generic programs run exactly as before.
- Translation and typing (`Spec/Semantics/Program.lean`): signature tokens are
  translated under the instance's type arguments (`RuntimeType.ofTokenWith`;
  `ofToken`, `ofSlotToken`, `ofSignature`, `vectorElemType` keep their names
  as the closed forms); a struct instance has one type argument per type
  parameter, each a value type (`structType?`: a struct type with a
  reference among its type arguments is not a type, so no value of a value
  type holds a reference; requested by the proof owner, 2026-11-02, and no
  type the semantics builds is affected), its field types are the declaration's tokens
  under its type arguments, and its abilities are the declared ones granted
  only as its non-phantom type arguments allow (`RuntimeType.abilities`,
  `argumentAbilities`). Closed types of the program (`RuntimeType.wellFormed`)
  have their arities and satisfy their constraints all the way down, phantom
  arguments included.
- The step (`Spec/Semantics/Step.lean`): `CallGeneric` and the generic forms
  of `Pack`, `Unpack`, the variant forms, `TestVariant`, the field and variant
  field borrows and the five global storage operations, each as its
  non-generic form at the instance. A generic form names a generic entity with
  one type argument per type parameter, a non-generic form a non-generic one
  (`badOperand`, L9); a callee's definition has the handle's arity
  (`linkMismatch`, K1); type arguments satisfy the definition's constraints
  (`constraintViolation`). The transaction's type arguments are checked as a
  call site's (`initialState`, `Program.typeArgsOk`). R7 for type
  specialization: type arguments may grow without bound under cyclic
  dependencies, and a VM that bounds them stops with a limit, which the
  stopping relation allows at every state.
- The ghost layer annotates every generic form as its non-generic form, and
  D3a's events record instances; D2–D4 are unchanged and cover generic code
  ("under every instantiation", D3a).
- `Supported` (`Spec/Properties/Safety.lean`): tokens translate under any type
  arguments of the declared count; natives may be generic, still with values
  only.
- K1 (`HandleResolution`): a function handle has its definition's number of
  type parameters, at least its constraints, and the same parameter and
  return types at every instance. K2 covers `CallGeneric`.
- C1: `typeAfter` for every generic form; consistency for every instance whose
  type arguments satisfy the constraints; `TypeCertificate.Parametric`, the
  certificate of an instance is the same tokens translated under its type
  arguments. `CertificatesHold` requires it.
- L7 (`Spec/Properties/Instantiation.lean`), stated as the book states it:
  from any function of the module at any type arguments, finitely many
  instances of the module's functions are reachable through its own calls,
  every call instruction counting. L11 (`Spec/Properties/Limits.lean`): the
  counts of struct and function definitions, variants, fields, parameters,
  return values, basic blocks and loop depth, and the size and depth of every
  signature and field token, against the recorded configuration; syntactic
  only.
- N2 and N2 for transactions are premised on the instance's type arguments
  satisfying its constraints. E1: a generic entry function is called at any
  closed type arguments satisfying its constraints; the accepted struct types
  `A` are instances.

Phase 4, stage B (function values):

- Types and values (`Spec/Semantics/Values.lean`): `RuntimeType.function
  params results abilities`, whose parameters and results are slot types
  ("the actual parameters of lambdas can be references", book,
  `functions.md`). `Value.closure fn mask captured params results abilities`:
  a function instance, the mask of its captured parameters (bit `i`, least
  significant first, captures parameter `i`), the captured values in
  parameter order, and the closure's type, recorded as a vector records its
  element type. `FunctionRef` moves here from `Program.lean`, unchanged.
  Closures are equal when they are the same function instance with the same
  mask and captured values ("based on the name of the underlying function",
  then "the captured values", book). `maskSplit`, `maskCompose` and
  `maskWithin` give the mask's meaning (`file_format.rs`, `PackClosure`,
  `CallClosure`).
- Assignability (`RuntimeType.assignable`, plan §5.2): a function type with
  more abilities to the same function type with fewer, `&` covariant in its
  target, otherwise equality. Conformance (`Value.conforms`, `allConform`,
  `conformsAll`) uses it; for types without function types it is equality.
  `Eq` compares values whose types are assignable to a common type
  (`RuntimeType.comparable`): equal up to a function type's abilities, at the
  top or under immutable references. The proof owner found the first version,
  which relaxed only top-level function types, rejecting two `&F` references
  whose targets differ in abilities (2026-11-02); the other comparisons of
  runtime types are of struct types, vector element types, function
  parameters and results, and declared types, which assignability keeps
  equal. A local's reference has the local's declared type as its target
  (`Step.borrowLoc`), not its value's, which may have more abilities.
- Abilities (`Spec/Semantics/Program.lean`): a function type has the abilities
  it carries; a closure's are `copy`, `drop`, and `store` if its target is
  storable (`public` or `#[persistent]`, not merely `entry`,
  `Program.storable`; language-owner ruling 2026-11-03, superseding the book's
  `entry` clause in "Denoting Function Values"), never `key`
  (`Program.closureAbilities`), intersected with its captured values' types'
  ("intersected with the abilities of all the captured context variables",
  book;
  `Program.closureValueAbilities`), which may exceed the captured parameters'
  declared types (corrected 2026-11-02 at the proof owner's report: the first
  version took the parameters' types, so a closure capturing a closure with
  more abilities than its parameter's type lost them). A function type is
  well formed without `key`.
- The step: `PackClosure` and `PackClosureGeneric` resolve, link, instantiate
  and check visibility as calls do, reject a mask bit beyond the parameters
  (`badOperand`) and a captured reference (new reason `capturedReference`),
  and pop the captured values. `CallClosure` checks the closure's type against
  its function-type operand (abilities at least the operand's), pops the
  remaining arguments, and enters the target with the mixed arguments,
  whatever its visibility; dispatch counts the target's module as active
  (R3) and fails under the module lock if that module is active (R4).
- The ghost layer: a closure's captured values are its node's children
  (occurrences, D2); `PackClosure` creates the closure, `CallClosure` consumes
  it (`Consumer.callClosure`, a legitimate consumption) and moves its
  captured values and the provided arguments into the target's parameters,
  using and passing the provided references as `Call` does (D4).
- D3c (`Spec/Properties/Encapsulation.lean`, part of `Encapsulated`): every
  closure was created by code its target is visible from, or was in global
  storage initially; natives and transactions create none
  (`Creator.MayCreateClosure`).
- D5 (`Spec/Properties/FunctionValues.lean`): in every reachable state,
  captured values hold no reference, every closure is well formed with
  abilities justified by its target and its captured values, and every
  closure in global storage targets a storable function; every `CallClosure`
  that continues enters its target with exactly its parameters, conforming.
  Theorem shape `FunctionValueSafety`, over admissible initial states.
- K2: `CallVisibility` covers `PackClosure` and `PackClosureGeneric`;
  `PersistenceResolution`: a handle carrying `persistent` names a storable
  function. `LinkedProperties` includes it.
- C1: `typeAfter` for the three instructions, and assignability wherever a
  value moves into a typed slot (`StLoc`, calls, `Ret`, `Pack` and the
  vector packs, `WriteRef`, `VecPushBack`); `Eq` takes comparable types.
  `TypeCertificate.Consistent` asks a successor's certified types to be
  supertypes of the instruction's result (`RuntimeType.assignableAll`), not
  equal to it: a closure's static type depends on the instance its captured
  values' types are at (a `T` capture has `T`'s instance's abilities), and a
  parametric certificate is one token for every instance (corrected
  2026-11-02 at the proof owner's report).
- L7 follows closure creation (`Bytecode.directCall?`). L5 and L9 are
  unchanged: already general.
- `Supported`: `Bytecode.inClosures`; natives name no function type
  (`NoFunctionTypes`); `Program.Supported` also requires every closure to
  target a function with code (`ClosureTargetsHaveCode`).

Natives with references (2026-11-03), written clean-room from the plan, the
book, the declarations and doc comments of `move-stdlib/sources` and
`file_format.rs`:

Language-owner rulings, 2026-11-03, implemented as given:

1. *Shared contract shapes, not per-native contracts.* The natives are
   described by three shapes (read, update, borrow), not one contract each.
   The first ruling also defined the eight natives of `std::vector` marked
   `#[bytecode_instruction]` exactly, as their instructions. *That half was
   replaced by a later ruling the same day:* the eight are **not** defined as
   their instructions. They are ordinary modeled natives of the matching
   shape, with no definition in the semantics: `length` read; `push_back`,
   `pop_back` and `swap` update; `borrow` and `borrow_mut` borrow; `empty` and
   `destroy_empty` take and return values only, exactly as before. The reason,
   in language terms: `#[bytecode_instruction]` tells the compiler to emit the
   instruction for a call, and neither the declarations nor their docs promise
   that a direct bytecode `Call` of these natives behaves as the instruction.
   So the spec assumes nothing of them beyond their shapes, which any behavior
   that meets them satisfies, failing included.
2. *A signer contains an address that can be borrowed* (`signer.move`: the
   runtime representation holds `account: address`, and "`borrow_address`
   borrows this inner field"), so `signer::borrow_address(self: &signer):
   &address` returns a reference to the address inside the signer its
   argument designates.
3. *Scope:* exactly these natives of `std`, with these shapes. Read:
   `vector::length`, `string::internal_check_utf8`,
   `internal_is_char_boundary`, `internal_sub_string`, `internal_index_of`,
   `bcs::to_bytes`, `bcs::serialized_size`, `cmp::compare`. Update:
   `vector::push_back`, `pop_back`, `swap`, `move_range`, `mem::swap`. Borrow:
   `vector::borrow`, `borrow_mut`, `signer::borrow_address`. Every other
   native with a reference parameter or result, or a function type in its
   signature, stays outside the subset.

The trusted changes, each with its reason:

- `Spec/Semantics/ModeledNatives.lean` (new): the table `modeledNatives`, with
  each native's module, name, type parameters, declared parameter and return
  tokens (as `move-stdlib/sources` declares them) and shape (`NativeShape`);
  and the identification (`CompiledModule.modeledNative?`,
  `Program.nativeShape?`). A native is modeled when its module is `0x1::m`,
  its name the entry's, and its declared signature the entry's: the same type
  parameters with the same constraints (none), and tokens matching
  `TokenPattern`s, struct types by module and name. Reason: ruling 3 asks for
  these natives with their declarations, and the binary format carries no
  `#[bytecode_instruction]` attribute (`FunctionAttribute`, `file_format.rs`),
  so module id, name and signature are what the bytecode offers. A native that
  matches by name only is not modeled. The language owners confirmed
  identification by exact declaration (2026-11-04): a native declaring other
  constraints, as `borrow<Element: copy>` would, is not modeled, and
  visibility and `entry` play no part.
- `Spec/Semantics/Values.lean`: `PathStep.signerAddress`, with
  `Value.child? (.signer a) .signerAddress = some (.address a)` (ruling 2).
  `withChild?` stays `none` on it: replacing the address would make a signer
  for another address, which no instruction may (D3b), and no reference to it
  is mutable. `Location.descend`, `Location.overlaps` (helpers).
- `Spec/Semantics/State.lean`:
  - `State.nativeView`: what a native sees, each reference argument replaced
    by the value it designates ("sees what its reference arguments
    designate, not the references themselves", ruling 3);
  - `StepResult.callNative` carries the arguments and the views;
  - `NativePlace`, and `NativeResult.returned values writes places`, with no
    default arguments so that no statement can silently mean "no writes";
  - new wrong reason `overlappingReferences`: "Move prevents from having two
    mutable references to the same value, so `left` and `right` references
    are always distinct" (`mem.move`; likewise `vector.move`), and Move is
    "strict in ensuring unique ownership of mutable references before writes"
    (book, `references.md`, "Ownership").
- `Spec/Semantics/Step.lean`, `call`: after every check a call makes
  (resolution, visibility, K1, type arguments, argument conformance), a
  modeled native suspends with its views, after checking that every reference
  argument is valid (`invalidReference`; plan §5.2: "Using an invalid location
  is `wrong`") and that no mutable reference argument overlaps another
  reference argument (`exclusiveArguments`, `overlappingReferences`); any
  other native with references stays `unsupported`. The instructions are
  unchanged.
- `Spec/Semantics/Run.lean`:
  - `Program.nativeResultFits`, shared by the semantics and `NativesRespect`:
    the update shape writes back one conforming value per mutable reference
    parameter and returns no places; the borrow shape writes nothing and
    returns one fitting place per reference result (`nativePlaceFits`: a
    reference parameter at least as mutable, a path that exists in what it
    designates, a declared type that is the result's target, or for `&` one
    assignable to it, `RuntimeType.placeType?`); every other native returns
    values only. Reason: ruling 3's shapes, stated defensively as
    `resumeNative` was;
  - `resumeNative P s f args`: a result outside the shape is `wrong
    .nativeContract`; an update-shape native's written-back values go through
    its mutable reference arguments (`writeBack`, `Program.writtenReferences`),
    each place changing in place (`Invalidation.mutated`, as `WriteRef`: "A
    write `*x = v` discards the value previously stored in `x`"); each place
    becomes a new reference (`placeReferences`, `Step.borrow`), interleaved
    with the value results by the return types (`nativeResultsIn`);
  - `StepRel.native` applies the native relation to the views.
- `Spec/Semantics/Trace.lean`:
  - a signer's address is a node inside the signer (`Value.children`,
    `nodes`, `IdTree.build`, `PathStep.index`), so that reading through the
    borrowed `&address` reads that node's identity (ruling 2);
  - `Shadow.nativeStep` gains the pre-state, the views, the writes and the
    places: reads of every reference argument's location, writes of those
    written back, uses of every reference argument, a `borrowed` event per
    place (derived from the argument reference), and the D2 events, with
    `NativeFlows` over the outputs, the value results then the written-back
    values. What a reference argument designates stays in place unless an
    update-shape native writes it back (`Program.nativeKeeps`): it is never
    consumed, and a node passed on from it is a copy. What a written-back
    argument designated is replaced, and its nodes not passed on are
    consumed;
  - `TracedStep.native` and `traceStep` follow. `Shadow.exec` is unchanged.
- `Spec/Properties/Safety.lean`: `CompiledModule.Supported`'s native clause
  admits a modeled native (`(m.modeledNative? d).isSome ∨` the old clause);
  `NativesRespect` asks every result to fit its shape
  (`Program.nativeResultFits`).
- `Spec/Properties/Natives.lean`: `NativeCallRespects` treats reference
  arguments the native does not write back as shared (passed on only with
  `copy`, never consumed; `Program.nativeKeeps`);
  `NativesRespectOwnership` quantifies over the new result form, with the flow
  over results then writes. The module docstring records the inspection
  decision (below).
- `Spec/Properties/NonVacuity.lean`: N3 states its old clauses over the new
  result form (`.returned vals [] []`) and adds one: a read-shape native whose
  value parameters and results are plain returns on conforming arguments as
  it sees them (`RuntimeType.seen`). Witness: natives fail, except plain
  natives and these, which return a new plain value per result (`false`,
  `0`, the empty vector) with fresh flows, consuming their plain value
  arguments and leaving what their references designate untouched. N3 is
  proved with this witness (`Proofs/NativeWitness.lean`).
- Docstrings only: `Theorems.lean`, `References.lean`, `Encapsulation.lean`,
  `Ownership.lean`.

Design choices, with reasons:

- *Overlapping arguments.* Two reference arguments of one native call where
  one is mutable and they overlap (`Location.overlaps`) make the call `wrong
  .overlappingReferences`, before the native runs. The rule covers a mutable
  and an immutable argument too: unique ownership of a `&mut` is required
  "before writes", and a native reading through the `&` while writing through
  the `&mut` would see a value the write changes. In the modeled natives the
  case does not arise: no native takes both kinds. The language owners
  confirmed the wider rule (2026-11-04).
- *Write-back.* Only an update-shape native writes back, through every one of
  its mutable reference arguments; a borrow-shape native with a mutable
  argument (`vector::borrow_mut`) leaves the value there in place
  (`Program.writtenReferences`, `Program.nativeKeeps`). Each place is written
  as `WriteRef` writes it, with the same invalidation. The old value is not
  discarded by the semantics: where it goes is the flow's business, and D2
  judges it. No `drop` is required (`mem::swap` moves both values).
- *Places.* A place's type is the declared type at its path inside the
  argument's target (`RuntimeType.placeType?`), not the type of the value
  there: a mutable reference must have exactly the declared type, as
  `borrowLoc` gives a local's declared type, or a write through it could
  store a closure with fewer abilities than the field's type. An immutable
  one may be covariant. Mutable results need mutable arguments ("never more
  mutable than that parameter").
- *Reference events of a native.* Every reference argument's location is read
  (the native sees the whole value there), every one written back is written,
  and every one is used. No native call has `entered` or `frameEnded` events:
  a native has no frame and runs within one step. A consequence of seeing
  whole values: `vector::length` and `vector::borrow` *read* the vector where
  `VecLen` and `VecImmBorrow` only *inspect* it, so D4b′ forbids calling them
  while a `&mut` into an element is still to be used, and the instructions
  allow it (tested). The language owners ruled this stricter reading intended
  (2026-11-04): there is no fourth access kind for natives, and borrow-shape
  natives, `signer::borrow_address` included, read their arguments' locations.
- *D3 and inspection* (plan §16, question 2). `bcs::to_bytes`,
  `bcs::serialized_size` and `cmp::compare` read values of any type. No
  exception and no policy entry is needed. D3 is about integrity, not
  confidentiality (language-owner ruling, 2026-11-04): who creates, takes
  apart, and runs the privileged operations on values of a type, not who reads
  them. A read-shape native does none of these to another module's type (it
  writes nothing, creates only what every native may, consumes nothing it saw
  through a reference, records no `privileged` event). And the language makes
  such observation legitimate: "every Move value must be serializable" (book,
  `references.md`), BCS "is the binary encoding for Move resources"
  (`bcs.move`), `Eq` already compares other modules' values (plan §7, D3), and
  W2b's invariants are "insensitive to ... equality comparisons". Natives that
  create or take apart other modules' values still need exceptions.
- *Natives and the ghost's signer.* Every signer now has two nodes, itself and
  its address. The address is plain data, so no D2 or D3 check depends on it.

No statement is weaker for what it covered before:

- `Supported` admits more modules, never fewer, so every theorem premised on
  it covers more programs.
- `vector::empty` and `vector::destroy_empty` are unchanged: they take and
  return values only, are not modeled, and are natives as they were before
  this step (next item).
- For a native that takes and returns values only, `State.nativeView` is the identity,
  `exclusiveArguments` holds, `resumeNative` performs the same checks in the
  same order and pushes the same values, `nativeStep` yields the same shadow
  and events, and `NativesRespect`, `NativeCallRespects` and
  `NativesRespectOwnership` say what they said (the old relation's
  `.returned vals` is `.returned vals [] []`, and a native that takes values
  only may return nothing else).
- The instructions, `Shadow.exec` and every non-native step are unchanged.
- The ghost's extra node per signer is an address, which has `copy`, `drop`
  and `store` and is no struct, signer, closure or reference: every D2, D3
  and D4 check holds of it trivially, and every step the ghost accounted
  before it still accounts (the existing tests are unchanged).
- N3 keeps every clause and adds one.

Tests (`Tests/Semantics.lean`, "Natives with references"): mock `std`
modules declaring the natives as the sources do. The vector natives with
references work under a sample relation that meets each shape (`push_back`,
`borrow`, `borrow_mut` with its place at element `i` and a write through the
result, `pop_back`, `swap`, `length`, `move_range`, and the out-of-bounds
failures), with the identities its flows give, and under the relation that
only fails. The flows meet `NativeCallRespects`, checked on every native of the
cases (corrected 2026-11-04: they passed a written-back vector's node on
though its value changed; a vector a native writes back is now a new node, its
elements keeping their identities); every step is accounted, with no D2, D3 or D4 violation; their
shape violations are `wrong .nativeContract`; `length` under a live element
`&mut` is a D4b′ violation where `VecLen` is not. `empty` and `destroy_empty`
behave as any value native: any conforming result, and `wrong
.nativeContract` for anything else.
`signer::borrow_address` yields a reference that reads the held signer's
address, and its identity. `mem::swap` (also through `mem::replace` at a type
parameter, into a held value) runs under a sample relation, moving
identities; `cmp::compare` compares another module's struct values without a
D3 violation. Each shape violation is `wrong .nativeContract`, overlapping
mutable arguments `wrong .overlappingReferences` (`mem::swap` of one local,
`move_range` within one vector), dangling ones `wrong .invalidReference`, and
a duplicating swap fails D2a and D2b. Natives matching by name but not
signature, and a table-like `borrow_box`, are not modeled, and its call is
`unsupported`. Kernel-checked: the mock `std::mem` is `Supported`, and the
module declaring `borrow_box` is not.

Table natives (2026-11-04), written clean-room from the rulings, the plan,
`aptos-stdlib/sources/table.move` and the existing spec:

Language-owner rulings, 2026-11-04, implemented as given:

1. *A table's entries are inside the table.* A `Table<K, V>` value holds its
   declared field (`handle`) and also its entries, a finite map from keys
   (values of `K`) to entries (values of `Box<V>`). The entries are part of the
   value for ownership, for its abilities and for locations; a reference to an
   entry designates a place inside the table, through a new path step.
   `contains_box` and `destroy_empty_box` read; `add_box` and `remove_box`
   update, writing back the table; `borrow_box` and `borrow_box_mut` borrow,
   returning a place inside the table. Packing a `Table` gives a table with no
   entries. Reason given: `table.move` represents items as separate global
   state items tracked by the handle, which agrees with entries inside the
   value as long as no two tables share a handle, which ruling 2 ensures.
2. *A condition on the table module.* The six natives are modeled only in a
   program whose `0x1::table` creates a `Table` only by packing the result of
   a call to its native `new_table_handle`, never unpacks one, packs one only
   at a `V` with `store`, and calls each box native with `B = Box<V>`, checked
   on that module's bytecode.

   *Refinements, 2026-11-04.* The language owners stated ruling 2's intent:
   the model agrees with the implementation for every program the condition
   admits. Under that intent the same condition gains three syntactic checks,
   which `table.move` satisfies: 5. `0x1::table` declares `Table` exactly as
   `table.move` does, with exactly the ability `store` and a single field
   `handle: address`; 6. the module never borrows a field of a `Table`
   mutably, generic or not; 7. `new_table_handle`, the six box natives and
   `drop_unchecked_box` are private (neither public nor friend), as
   `table.move` declares them. Rules 5 and 6 close a gap the four checks left:
   a module declaring `Table` with `copy` or `drop`, or rewriting `t.handle`,
   could let two tables share a handle or drop a table of entries without
   `drop`. Rule 7 makes rules 1 and 4, checked on the module's own code, cover
   every call of these natives. Keys do not count for a table's abilities
   (confirmed the same day; design choices, below).

   *Modeling assumption* (language owners, 2026-11-04). `table.move` gives
   each table "a unique handle". Handle uniqueness is the assumption under
   which modeling the entries inside the table value, and comparing tables by
   their entries, agrees with the implementation, which keeps the items under
   the handle and compares the handle; the table rules keep it for the
   programs they admit. The spec does not ask `new_table_handle` for fresh
   handles: handles play no part in the model.
3. *Destroying tables.* `destroy_empty_box` only reads and promises nothing
   about emptiness. `drop_unchecked_box`, a native on values, consumes the
   table with its entries; D2 counts this as the native consuming them, which
   the native policy must allow. The spec does not require the table to be
   empty.
4. *Contracts promise shapes only.* Any call may fail; the spec does not say
   which entry a key selects, nor that `remove_box` returns what `add_box`
   stored.

The trusted changes, each with its reason:

- `Spec/Semantics/Values.lean`:
  - `Value.table s fields keys entries` (ruling 1): a value of the table type
    (`StructIdOf.isTable`, `0x1::table::Table` at any instance) is a table,
    with its declared fields and its entries, `keys[i]` selecting
    `entries[i]`. Its type is `.struct s`; equality compares type, fields, keys
    and entries. Keys and entries are parallel lists rather than a list of
    pairs, so that `Value` stays nested through `List Value` only, as before;
  - `PathStep.tableEntry i`, entry `i` of a table (ruling 1: "through a new
    path step"); `child?` and `withChild?` reach a table's fields and entries;
  - `stdAddress` moved here from `ModeledNatives.lean` (same name), with
    `tableModuleId` and `tableEntryType` (`Box<V>`).
- `Spec/Semantics/Program.lean`:
  - `SignatureToken.abilitiesIn` and `argumentAbilities` moved here from
    `Spec/Properties/Declarations.lean`, unchanged: the table rules use them;
  - the table rules, `CompiledModule.followsTableRules` (ruling 2; below), and
    `Program.tablesHoldEntries`, which holds when the program's `0x1::table`
    keeps them;
  - `Value.abilities`: a table's entries count, as a container's contents
    (ruling 1); its keys do not (below);
  - `Value.wellFormed`: a struct value of the table type is not well formed
    (a table type's values are tables); a table is well formed when its
    fields conform, it has one key per entry, and, if it has entries, its
    program keeps the table rules, its keys conform to `K` and are distinct
    ("a finite map"), its entries conform to `Box<V>`, its type has no `copy`,
    and the entries' type, as a container's content
    (`AbilitySet.forContainer`), has every ability the table's type has
    (below).
- `Spec/Semantics/ModeledNatives.lean`: the six natives, with their
  declarations as `table.move` writes them; `TokenPattern.structInstantiation`
  for the generic structs `Table` and `Box` (a struct instantiation token
  matches by module and name, one type parameter per argument, and its
  arguments); `ModeledNative.admittedIn`, ruling 2 as part of the
  identification (`CompiledModule.modeledNative?`).
- `Spec/Semantics/Step.lean`: `Pack` of a table type makes a table with no
  entries (ruling 1); `Unpack` of a table pushes its fields.
- `Spec/Semantics/Run.lean`: `RuntimeType.placeType?` gives an entry's
  declared type, `Box<V>` in a `Table<K, V>`, so that `nativePlaceFits` checks
  borrowed entries as other places.
- `Spec/Semantics/Trace.lean`: a table is a node whose children are its fields,
  then its entries (`Value.children`, `nodes`, `IdTree.build`); keys are not
  nodes. `Location.indices` became `Location.indicesIn`, which follows a path
  through the value at the location's root (`Value.indicesOf`,
  `Value.childIndex`: a table's entry `i` is its child after its fields), and
  `Shadow.tree?` and `withTree?` use it. For every value without a table the
  indices are what they were.
- `Spec/Properties/Natives.lean`: `NativePolicy.destroys` (ruling 3: the
  policy must allow `drop_unchecked_box` to consume the entries; default none);
  `Consumer.Legitimate` also accepts a native's whole consumption of a value of
  a type the policy lets it destroy; tables are created, consumed and taken
  apart as struct values of their type.
- `Spec/Properties/FunctionValues.lean`: the new constructor in
  `Value.isClosure` and `closureOk`.
- Docstrings only: `Safety.lean`, `Theorems.lean`, `Ownership.lean`,
  `Encapsulation.lean`, `NonVacuity.lean`, `Declarations.lean`.

The table rules (`CompiledModule.followsTableRules`), on the bytecode of the
program's `0x1::table`, rule by rule:

1. "creates a `Table` only by packing the result of a call to its native
   `new_table_handle`": every `PackGeneric` of `Table` directly follows a `Call` or `CallGeneric` of the module's own
   `new_table_handle`, which is a native of the module, at an offset no jump
   targets, so that the packed value is that call's result; no non-generic
   `Pack` names `Table`;
2. "never unpacks a `Table`": no `Unpack` or `UnpackGeneric` of it;
3. "packs a `Table` only at a value type argument `V` that has `store`": the
   second type argument of each `PackGeneric` of `Table` has `store` under the
   constraints of the packing function's type parameters
   (`SignatureToken.abilitiesIn`);
4. "calls each box native with `B = Box<V>`": every call of one of the
   module's own natives whose names end in `_box` (`drop_unchecked_box`
   included: it takes `B` too) is a `CallGeneric` whose third type argument is
   the module's `Box` at its second;
5. "declares `Table` as `table.move` does": the module defines a struct named
   `Table`, and every such definition declares exactly `store` and the single
   field `handle: address` (field name and type);
6. "never borrows a field of a `Table` mutably": no `MutBorrowField` or
   `MutBorrowFieldGeneric` whose field handle's owner is `Table`. Immutable
   borrows of `handle` are allowed (tested);
7. "`new_table_handle`, the six box natives and `drop_unchecked_box` are
   private": every function definition of the module with one of these names
   has private visibility.

Design choices, with reasons:

- *Where ruling 2 lives: in the identification.* The module the rules are
  about is the one that declares the natives, so `modeledNative?` checks
  them (`ModeledNative.admittedIn`). A program breaking them declares table
  natives with references that are not modeled: its module is not
  `Supported`, every theorem excludes it, and a call is `unsupported`. No
  theorem shape changes.
- *A table holds entries only where the table rules hold*
  (`Program.tablesHoldEntries`, in conformance). Entries enter a table only
  through the box natives, modeled only then, and otherwise only with initial
  values. Without this, a program whose `0x1::table` unpacks tables, and
  declares no box native (so `Supported`), would lose the entries of an
  initial table at `Unpack`, a destruction D2 could not account for.
- *Entries never take an ability away from the table's type* (accepted by the
  language owners, 2026-11-04). The verifier gives `Table<K, V>` its declared
  abilities whatever `V` is (`V` is phantom), while ruling 1 makes the entries
  count for the value's abilities. Conformance therefore asks the entries'
  type, as a container's content, to have every ability the table's type has.
  Under the table rules `Table` declares exactly `store` (rule 5), so this is
  the entries' type having `store`, which rule 3 gives every table
  `table::new` creates. The clause is kept on the value so that D1 holds
  independently of the rules: whatever a module declares, a table with
  entries keeps the abilities of its type, and dropping or copying one is
  never `wrong` in verified code.
- *A table whose type has `copy` holds no entries.* Ruling 1's model holds "as
  long as no two `Table` values share a handle", and a copy shares it. Rule 5
  excludes such a `Table` from every admitted program; the clause, like the
  previous one, keeps conformance meaningful on its own, and keeps tables with
  entries out of transaction arguments: the transaction layer accepts only
  struct types with `copy`, and the payload gives a struct its fields, not
  entries.
- *Keys are part of the value for equality only* (confirmed by the language
  owners, 2026-11-04). They are not nodes (no identity), no reference
  designates one, and they take no ability away from the table. `Table<phantom K: copy + drop, ...>` does not ask `K` for
  `store`, and `table::new` makes a storable table for any such `K`: were keys
  counted, storing a table with a key of a type without `store` would be
  `wrong` (`MoveTo` checks `key` all the way down), falsifying D1. Being
  `copy` and `drop`, keys are outside D2; no native returns a key, so no key
  comes back out as a value. A consequence: D5d does not see a closure used as
  a key (Open, below).
- *Path step and ghost children.* `tableEntry i` is entry `i` in the order the
  table holds them; the ghost places a table's entries after its fields, so a
  location's path is followed in the value at its root to find node indices
  (`Location.indicesIn`). Which entry a key selects is the native's (ruling 4).
- *Write-back.* `add_box` and `remove_box` write back the whole table, as
  every update-shape native does: every reference into it, to an entry or
  inside one, becomes invalid (`Invalidation.mutated`), as a vector's elements
  do when it grows or shrinks. A reference to an entry held across an
  `add_box` is used invalidly (`wrong .invalidReference`), and D4b′ flags the
  write over it.
- *Reads.* As for every native, `contains_box`, `destroy_empty_box`,
  `borrow_box` and `borrow_box_mut` read the whole table, so calling
  `contains_box` while a `&mut` into an entry is still to be used is a D4b′
  violation (tested), as `vector::length` under a live element `&mut` is.
- *Destroying.* `NativePolicy.destroys f t`: native `f` may consume a value of
  type `t` whole (nothing inside it passed on) although `t` lacks `drop` and
  `f` the privileges of `t`'s module. `drop_unchecked_box` consumes the table,
  its handle, its entries and what they hold; `Table` and `Box` are its own
  module's, so only values of other modules' types without `drop` inside the
  entries need the exception. `destroy_empty_box` is a read-shape native and
  promises nothing (ruling 3).
- *`Unpack` of a table* pushes its fields: it can only run in a program whose
  table module breaks rule 2, where tables hold no entries.

No statement is weaker for what it covered before:

- Before this step a value of `0x1::table::Table` was a struct value; it is now
  the table with the same fields and no entries, which conforms, has the same
  abilities, has the same ghost nodes and identities, and behaves the same
  under every instruction (`Pack`, `Unpack`, field borrows, reads, writes,
  equality). Every program, state and execution covered before is covered
  with this one representation change, and tables with entries are added.
- `Supported` admits more modules, never fewer: the natives that become
  modeled were natives with references, outside the subset.
- `NativePolicy` gains a field whose default grants nothing: every policy of
  before is one of now, with `destroys` false, under which
  `Consumer.Legitimate`, `NativeCallRespects`, D2 and D3 say what they said.
- `Location.indicesIn` agrees with the old `Location.indices` on every value
  without a table, and a shadow that fits its state has a tree at a root
  exactly where the state has a value, so `Shadow.tree?`, `withTree?` and
  every ghost step of before are unchanged.
- `SignatureToken.abilitiesIn` moved files, unchanged; L5 and L6 read the
  same.
- N3 is unchanged as stated; its witness meets the new contracts (the
  read-shape table natives return new plain data at plain key types), and the
  update and borrow shapes are still not asked to return.
- The instructions other than `Pack` and `Unpack` are unchanged, and those
  two only for the table type.

Tests (`Tests/Semantics.lean`, "Table natives"): a mock `0x1::table` declaring
`Table`, `Box` and the natives as `table.move` does, with `table.move`'s
functions calling them (`new`, `add`, `borrow`, `borrow_mut`, `contains`,
`remove`, `drop_unchecked`, `destroy_known_empty_unsafe`), and `0x2::coin`.
The identification of each native, and near misses: `K` with `copy` only, `V`
with `store`, no `B`, another address, another module name, and modules
each breaking one table rule (a forged handle, a jump to the pack, an unpack, a
pack at a `V` without `store`, `add_box` at `B = V`), whose natives are not
modeled and whose call is `unsupported`, and seven more breaking rules 5 to 7
(`Table` declared with `drop`, with `copy`, with its field named otherwise,
with a second field; `t.handle = h` through a mutable field borrow; `add_box`
public, `new_table_handle` friend), while an immutable borrow of `handle`
keeps the rules. Under a sample relation that meets
each shape: add, then borrow, then remove; a write through a borrowed entry,
read back; `contains`; failures; the table written back, with its
identities; `NativeCallRespects` for `add_box` and `remove_box` with their
flows. Every step is accounted, with no D2, D3 or D4 violation, under that
relation and under the one that only fails. `drop_unchecked_box` of a table of
`u64`s is accounted with no exception; of a table holding a coin, D2b and D2c
fail under the default policy and hold under one that lets it destroy coins.
D4: `borrow_box` reads the table and derives the entry reference from the
table reference; `contains_box` under a live `&mut` into an entry is a D4b′
violation; `add_box` under one invalidates it. Each shape violation is `wrong
.nativeContract`, a written-back table with a key twice or a plain struct value
included. Conformance: entries only with distinct keys, one key per entry, an
entry type with `store`, and under the table rules.

### Waiting on a verifier fix

- **L6, phantom positions behind references.** The model's acceptance does
  not establish L6's phantom-position rule for field types that mention a
  reference, so `localProperties_of_phantom` takes that rule as a hypothesis
  (`PhantomUnderReferences`). A fix of the Rust verifier is tracked
  separately. The model already has it behind a switch,
  `SignatureV2.phantomThroughReferences` (off, as in the verifier today), and
  `localProperties_of_switch` proves every local property unconditionally
  once the switch is on in the production configuration. When the fix
  lands: add its `VerifierConfig` field to `Spec/Config.lean` and the
  recorder, define the switch from it, re-record the configuration, and add
  the fix's test modules to the model-agreement fixtures.

### Divergence ledger (public part)

Transcription defects (plan §10.1), fixed in the model:

- 2026-10-02, `signature_v2`: the model used `Ability::required_by` where
  Rust uses `AbilitySet::requires` (vector elements, struct type arguments,
  struct fields). It rejected `struct S has store { x: u64 }` and accepted
  `struct K has key { s: signer }`, both unlike Rust. Fixtures
  `passes/fieldstore.masm`, `fieldkey.masm`, `fieldkeyvec.masm`.
- 2026-10-04, `reference_safety`: Lean's `do` notation evaluates
  `(← isWritable id)` before `mut_ && …`, so the model called
  `is_writable`, which asserts a mutable reference, where Rust's `&&`
  short-circuits. The model panicked on `VecLen` and `VecImmBorrow` through
  an immutable reference; the join had the same pattern around `leq`.
  Fixture `refs/refsok.masm`.

### Open

- Later phases: natives beyond the modeled ones need state (aggregators and
  storage slots reach storage no argument designates), dispatch,
  and contracts for function values (a native that evaluates a closure is
  dynamic dispatch, R4); closures over natives; E1 needs scripts.
- Questions for the spec owners from natives with references (2026-11-03):
  - Whole values (settled 2026-11-04): the language owners ruled that the
    stricter reading is intended. A native call reads the whole value at each
    reference argument's location, so `vector::length`, `vector::borrow` and
    `vector::borrow_mut` read the vector where `VecLen`, `VecImmBorrow` and
    `VecMutBorrow` only *inspect* it (its length), and D4b′ rejects a direct
    call of `length` while a `&mut` into an element is still to be used,
    which it accepts for `VecLen` (tested). There is no fourth access kind for
    natives, and no inspection is recorded for natives that look only at a
    length. Compilers emit the instructions for these calls, so only
    hand-written bytecode makes the calls directly. The spec already did so
    (`Shadow.nativeStep`).
  - Overlapping arguments (settled 2026-11-04): the language owners confirmed
    the wider rule: a mutable reference argument of a native may overlap no
    other reference argument, immutable ones included
    (`overlappingReferences`), though the earlier ruling and the sources speak
    of two mutable references. The spec already did so
    (`exclusiveArguments`).
  - Borrow-shape reads (settled 2026-11-04): the language owners confirmed
    that a borrow-shape native reads its arguments' locations (it sees them),
    where a field borrow records only a use, `signer::borrow_address`
    included, for which a read and an inspection are equivalent: nothing
    mutable can be inside a signer. The spec already did so
    (`Shadow.nativeStep`).
  - Identification (settled 2026-11-04): the language owners confirmed
    identification by exact declaration: a native is modeled only if its
    declared type-parameter constraints are exactly the entry's (none). A
    standard library that declared, say, `borrow<Element: copy>` would not be
    modeled. Visibility and `entry` are not part of it (the `string` natives
    are private). The spec already did so (`CompiledModule.modeledNative?`).
  - N3 does not ask the update and borrow shapes to return: their natives are
    exercised by the tests with relations that meet the contracts. Asking it
    in general would tie N3 to particular natives (an `Ordering` value exists
    only if `std::cmp` declares a constructible one). Should N3 ask it?
  - D3 and inspection (settled 2026-11-04): the language owners ruled that D3
    is integrity, not confidentiality: encapsulation is about who may create,
    take apart or change a value of a module's types, not who may read it. So
    the inspection decision stands: D3 needs no exception for read-only
    natives such as `bcs::to_bytes` or `cmp::compare` that see private fields
    (plan §16, question 2).
- Questions for the language owners from the table natives (2026-11-04):
  - Settled 2026-11-04: the abilities `Table` declares and writes of its
    handle, by rules 5 and 6; calls of the box natives from other modules, by
    rule 7; fresh handles, by the modeling assumption of handle uniqueness
    (all above); keys count for equality only, as the spec has it (design
    choices, above).
  - Closures as keys. A key may be a closure (a function type with `copy` and
    `drop`): may a closure that is not storable be the key of a table in
    global storage? D5d ("every closure in global storage targets a storable
    function") does not look at keys, which are not nodes; no key ever comes
    back out as a value.
- E1: struct visibility (`public`) has no field in the binary format, so the
  struct types the transaction layer accepts are a parameter (`A`) that E1
  only constrains by the book's rule. Entry functions a transaction calls
  return nothing: the transaction layer refuses any other before it runs,
  and `EntryInitial` requires it (`P.returnTypes f = some []`; language-owner
  ruling 2026-11-03).
- Settled in Phase 4, stage A: K1 compares type-parameter counts and
  constraints, conservatively as for struct handles (a handle may require
  more constraints than its definition); a non-generic form naming a generic
  entity, or a generic form a non-generic one, is `wrong .badOperand`, and a
  definition of another arity than the handle `wrong .linkMismatch`. Struct
  handles may be generic.
- Questions for the spec owners from Phase 4, stage A:
  - Conformance does not check that a value's type satisfies its
    definition's constraints; the semantics checks constraints where it
    instantiates (`CallGeneric`, the generic struct forms, the entry's type
    arguments), and the rest is L5. Should conformance, and so
    `GlobalStorage.wellTyped` and the natives' results, also require
    `RuntimeType.wellFormed`? It would change no non-generic execution.
    Settled in part: a struct type resolves only if its type arguments are
    value types (`structType?`), so conformance already excludes references
    as type arguments, at any depth a value reaches; constraints and arities
    of type arguments that no value reaches (phantom ones) remain
    unchecked.
  - A reference as a type argument is `unsupported "type arguments"`, as
    other untranslatable tokens are, rather than a reason of its own.
  - A generic form with an empty instantiation is `badOperand` (L9 asks for at
    least one type argument), though it would be harmless on a non-generic
    entity.
  - K1 for function handles requires the definition's constraints to be
    included in the handle's, not equal to them: the plan's wording
    ("whose type-parameter constraints the handle's include").
  - L7 is stated semantically (finitely many reachable instances), not as the
    instantiation graph; `Call` of the module's own non-generic functions and
    its own natives count as calls. Calls into other modules are K4's.
  - L11's measured objects are chosen from the configuration's names, not
    from the verifier: fields per struct and per enum variant (not the total
    of an enum); parameters and returns per function definition (not imported
    handles); type size as a node count, which the plan's weighted size bounds
    if every node weighs at least one (the weights are not part of the
    language); depth counted from one; loop depth as the number of natural
    loops containing an instruction; the tokens of the signature pool and of
    field types (not constants). `maxGenericInstantiationLength`,
    `maxPushSize`, the back-edge, script and meter limits are not stated.
  - C1's parametricity (`TypeCertificate.Parametric`) fixes the static
    certificate as slot tokens of the function's module.
  - Natives receive their type arguments through `FunctionRef`; the native
    policy may therefore grant privileges per instance.
- Questions for the spec owners from Phase 4, stage B:
  - Storability (settled 2026-11-03): the language owners ruled that a
    function is storable exactly when it is `public` or `#[persistent]`;
    `entry` alone does not count, and the book's `entry` clause is wrong.
    `Program.storable` follows the ruling. Still open: how a module learns
    that an imported function is storable, other than by a `persistent`
    attribute on its handle (`PersistenceResolution`).
  - Masks (settled 2026-11-03): the format bounds only the number of
    captured parameters ("`m <= n`"); a bit at or beyond the target's
    parameter count names no parameter and is `badOperand`, as the
    semantics has it.
  - Equality of closures (settled 2026-11-03): two closures are equal exactly
    when they are over the same function instance (type arguments included),
    with the same mask and the same captured values, as the semantics has
    it; closures over one function at different type arguments, or with
    different masks and the same captured values, are not equal.
  - A struct field of a function type "inherits the abilities of the struct"
    in source (book, "Function Types"). The spec takes a field's type as the
    bytecode writes it, so a field type without `store` in a struct with
    `key` breaks L6; we assume the compiler writes the inherited abilities
    into the token.
  - Closures over natives are outside the subset (`ClosureTargetsHaveCode`,
    and they are not well formed): the native contracts and the ghost layer
    describe native calls whose arguments come from the stack only. This
    includes the modeled natives (2026-11-03).
  - Natives whose signatures name a function type are outside the subset;
    a native at function-type type arguments may receive closures and pass
    them on, but may create none (`Creator.MayCreateClosure`).
  - A function type with `key` is not well formed (`RuntimeType.wellFormed`)
    and no closure has `key`; the type's abilities are still the ones it
    carries, so such a type is uninhabited rather than rejected where it is
    written.
- Points the spec owners flagged, checked against Rust: L5 admits
  `StructInstantiation(h, [])` for a non-generic handle (two spellings of one
  type), but the production deserializer rejects an instantiation with no
  type arguments (`MALFORMED`), so it cannot come from bytes (A4); L6's
  containment counts phantom type arguments, as the recursive-struct check
  does. L5's `key = false` on function types is not stated by the book; the
  production verifier rejects it.
- Struct-API attributes (version 10) are not stated in L9: their meaning
  needs agreement with the language owners.
- `file_format.rs` doc comments that disagree with the format or the book:
  addresses are described as 16 bytes; `Shl`/`Shr` say the shift amount is
  `u64` (it is `u8`) and swap left and right; `ImmBorrowVariantField` lists
  an instantiation index as its operand; `TestVariantGeneric` has no
  `static_operands`; `Pop` says the value must be copyable (it needs
  `drop`); `Eq` says equality is defined only on primitives, vectors and
  references (the book includes structs); `VecUnpack` ends with `elem_n`
  (`elem_(n-1)`); `UnpackVariant` mentions `struct_ref` and `variant_field`;
  `VecPack` calls ill-typed operands an abort. The spec follows the format
  and the book.
- `CompiledModule.StructFieldInformation.allFields` is declared inside
  `namespace CompiledModule`, which looks unintended.
- Questions for the spec owners from Phase 3:
  - R3's scope (settled 2026-11-03): the lock covers all five global storage
    operations, `exists` and `move_to` included (the book's reentrancy
    example names only `borrow_global`, `borrow_global_mut` and
    `move_from`), and is checked before existence (R5) for each: a locked
    `MoveFrom` of a missing resource fails as locked, not as missing, and a
    locked `MoveTo` onto an existing resource as locked, not as existing.
    The spec already did so (`unlocked`). MonoVM follows this order; the
    existing Move VM reports the existence failure first for `MoveFrom` and
    `MoveTo`, a known divergence in the failure reported (both are
    failures).
  - `Exists` requires `key`, per the book; `file_format.rs`'s `Exists`
    check does not assert it.
  - The semantics checks a reference's validity only where it accesses the
    location. Should copying, moving, freezing, passing or returning an
    invalid reference also be `wrong`, or only a D4a violation?
  - The conservative invalidations: `push` invalidates references to
    elements; overwriting a local invalidates what was inside its old value
    but not the local itself.
  - `&mut T` does not conform to `&T`.
  - `acquires` is static machinery for D4; it is not checked dynamically.
  - D4's definitions depart from the plan's wording in two places, both
    forced by ordinary compiled code: a copy of a reference is the same
    reference, and a live child keeps its parent's borrow alive only on the
    child's own location (`Spec/Properties/References.lean`).
  - Should `MoveFrom` also count as making a slot dead for D4e? It is
    already a direct write for D4b′ and D4c.
- Human sign-off of `Spec/` (plan §10.5). `Spec/Config.lean` mirrors the Rust
  configuration and is outside the clean-room rule by nature.

## Layout

| Path | Trust | Contents |
|---|---|---|
| `Spec/Syntax.lean` | trusted | Abstract syntax, following `file_format.rs` |
| `Spec/Config.lean` | trusted | The recorded production configuration (A3) |
| `Spec/Semantics/` | trusted | Values, programs and resolution, the modeled natives (`ModeledNatives.lean`), state and outcomes, the deterministic `step`, executions, the ghost layer |
| `Spec/Properties/` | trusted | L, K, E, D, N, C statements, native contracts, and the theorems' shapes |
| `Model/` | untrusted | Transcription of the Rust verifier, bugs included |
| `Proofs/` | untrusted | Proofs, checked by the Lean kernel |
| `Import/` | A4 | Decoders for the exporter, config recorder, and oracle JSON |
| `Tests/` | — | Semantics, import, config, differential, model-agreement and review tests, axiom audit; the framework check (`Framework.lean`, N4) |
| `exporter/` | A4 | Rust: production deserializer or `move-asm` → JSON |
| `config-export/` | A3 | Rust: records `aptos_prod_verifier_config` |
| `vm-oracle/` | — | Rust: runs calls on the legacy VM and MonoVM, records the Rust verifier's decisions, on `.masm`, `.mv` and release-bundle inputs |

## Build and test

Lean `v4.32.2`, as everywhere in this tree (`./scripts/dev_setup.sh -p -l`
from the repository root).

```bash
lake build
lake test        # semantics, review, import, config, differential, model agreement, axioms
cargo test -p move-bytecode-lean-export
scripts/check-framework.sh   # N4: the model on the framework's release bundle
```

`scripts/check-framework.sh` exports the modules of
`aptos-move/framework/cached-packages/src/head.mrb` with the Rust verifier's
decisions (`Tests/framework/head.spec.json`), into `.lake/framework-check/`,
and runs `Tests/Framework.lean` on them. The check fails if:
- the model rejects a module, or decides one differently from Rust;
- the module ids are not distinct;
- the framework's `0x1::table` breaks the table rules;
- fewer modules are covered than `Tests.Framework.minCovered`.

Otherwise it prints the coverage and what keeps modules outside the subset.
The export is not committed because it changes with every framework change;
raise `minCovered` when coverage grows.

The JSON fixtures in `Tests/fixtures/` are generated. Regenerate them after
changing the exporter, a `.masm` source, a differential spec, the
configuration, or either VM, and review the diff:

```bash
scripts/regenerate-fixtures.sh
```

A differential spec (`Tests/differential/*.spec.json`) names `.masm` inputs,
calls, and constants to decode; `vm-oracle` turns it into
`Tests/fixtures/diff_<name>.json`.

The tests read the fixtures while Lean elaborates them, which Lake's build
trace does not track. After regenerating fixtures, rerun the tests that read
them, for example `lake env lean Tests/ModelAgreement.lean`.
