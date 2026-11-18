# What prover-smoke covers

`scripts/prover-smoke.sh` is the inner loop for changes to the prover: kernel
laws, solver, specification layer. It elaborates VerexSL with the built Lean
libraries alone. It is not evidence for a frontend change: extraction,
printing and assembly of a source, the command line and the driver, and the
native executables are checked by the gate, which every commit still needs.
A rendered fixture covers what the prover does with VerexSL a frontend once
wrote, not the frontend. An open cell is a sentinel of a missing capability,
not coverage: it pins the first diagnostic of the rejection and admits only
verification failures of its target beside it; it does not pin the whole
list, and a green module does not say its open cells verify. Costs
complement these checks and replace none of them.

A fixture is in the suite because it is the smallest one that observes a
named thing: a cell of the shape matrix, a boundary control, a class of
regression, or a cost. The tables below say which. They do not claim the
full product of mechanisms and shapes.

## The tiers

| Tier (list in the script) | What it is | Where its coverage is listed |
|---|---|---|
| `SHAPES` | mechanism × type shape: one small function per cell, a positive and a pinned negative, open cells under `expectOpen` | `verex-ir/VerexSL/Tests/Shapes/README.md` |
| `IR_TESTS` | controls of the join scheduler, preparation, maps, frames, stated facts, diagnostics order, state labels | each module's header |
| `BOUNDARY_TESTS` | boundaries that are no type shape | the table "Boundaries" below |
| `RENDERED` | source fixtures as a frontend writes them, with the fixture's diagnostics | the table "Rendered fixtures" below |
| `CHECKS` | check fixtures of `verex-e2e-tests`: tables and their owners, module invariants, bit vectors, returned references, frames of function-valued parameters, an authored statement over a generated contract | each fixture's header |
| `COST_*` and the pins file | larger targets with pinned heartbeat costs | `scripts/prover-smoke-cost.pins` and the table "Costs" below |

## Classes of regression and the fixture that stands for each

The classes are those of the fixtures that regressed in the switch to plain
values while the `verex-ir` suite stayed green. "Observes" names the target
of the fixture that fails when the class regresses.

| Class | Subcase | Fixture and target that observes it |
|---|---|---|
| R1 an opaque specification function under a generic native's type arguments | at a struct, a scalar, the caller's parameter, a struct over it | `shapes OpaqueGenerics`: `struct_ok`, `scalar_ok`, `param_ok`, `instance_ok` |
| | at a type argument alone, the result of the caller's parameter type | `shapes OpaqueGenerics`: `named_ok`, `decoded_ok`, `decoded_named_ok` |
| | a nominal argument the caller constructs | `shapes OpaqueGenerics`: `constructed_ok` |
| | a serialization native | not covered: the intrinsic itself is not exercised |
| R2 a generic owner of a table; stored maps at a frame | owner at its own parameter | `shapes TableRows`: `owner_param_ok`; `check Storage/WrappedTables`: `slots_has` |
| | a structural diagnostic on a row's negative | `shapes TableRows`: `row_instance_bad`, `row_tagged_bad` |
| | the same through an authored proof file | not covered |
| R3 the interface authored Lean statements see | an instance of the carriers in a statement over a generated contract | `check Generics/Generics` |
| R4 an authored proof script over a vector mutation | | `check Examples/Quicksort` |
| R5 a generic `&mut Vector<T>` where the mutation ends | swap and remove at `T` | `shapes Vectors`: `swap_remove_param_ok` |
| | a removal through a borrow of a generic holder's field | `shapes Vectors`: `remove_field_param_ok` |
| R6 enums | a match that reads each variant's invariant clause | `shapes Enums`: `match_ok`, `branch_ok` |
| | a field read of an enum held by a resource | `shapes Storage`: `read_enum_ok` |
| | an optional result of a map, enum and vector representation | `shapes Enums`: `enum_replaced_ok`, `vector_replaced_ok`; `check Specifications/OptionalMapResults` (which also runs them) |
| | `Option::map` with a closure (Rust) | `rendered library_option_map`: `bump`, `constant` |
| R7 a struct built over vectors under an invariant | a vector of invariant-bearing structs | `shapes Construct`: `bag_ok` |
| | a generic enum built from the element popped out of a tested vector (an optional value from a vector, as the Move frontend lowers `option::from_vec`) | `shapes Vectors`: `from_vector_ok` |
| R8 trait-method closures (Rust) | `Fn`, `FnMut`, `FnOnce` receivers and their adapters; forwarding (`ensures_of` of the trait method) | `rendered fnonce_forward`: `shared`, `mutable`, `counter`, `forward` |
| | the components of a tuple result | `rendered fnonce_forward`: `pair`; `rendered static_fnonce`: `pair` |
| | a stateful callback under static dispatch | `rendered static_step`: `counter`, `toggle` |
| | a record result, a generic pair, a nested tuple, `FnMut` under static dispatch | not covered by a fixture of their own |
| | what the path must reject: a caller's clause that claims another result, or a state of the receiver the contract does not give | `shapes MethodRejections`: `twice_result_bad`, `twice_receiver_bad` (and the positive `twice_ok`) |
| | a method whose body does not meet its own contract | `shapes MethodRejections`: the namespace `stuck_method` (and the positive `flipping_method`) |
| | a caller that holds no contract of the method it passes on, or another function value's | `boundary VerexSL/Tests/ForwardedMethodEvidence` |
| | a method that panics; a wrong component of a tuple result | not covered: the first needs the `no panic` obligation of the Rust profile, the second a rendering of an `_errors` fixture |
| R9 generic function values | a generic higher-order function's own contract, one and two arguments | `shapes Closures`: `apply`, `apply_both`, `through_ok` |
| | partial application of a generic target | `shapes KnownClosures`: `generic_ok`, `generic_param_ok` |
| | the model behaviour of a function literal in a specification | `rendered generic_function_literals_model`: `via`, `guarded` |
| R10 a function value in storage | taken and invoked; published | `shapes KnownClosures`: `consume_ok`, `publish_ok` |
| R11 a table that is a call's result, built into a resource and published | | `shapes TableRows`: `publish_new_ok`; `rendered table_stored_resource`: `publish` |
| R12 a read after a write | another key, another resource, after a publish, after a call | `shapes StorageBorrows`: `read_other_key_ok`, `read_other_resource_ok`, `read_after_publish_ok`, `read_after_call_ok` |
| R13 an entry taken out of a stored map, unpacked, handed to opaque calls | the take, the unpack, a field compared with an opaque call's result, another decoded | `shapes TableTakes`: `take_decode_ok`, `take_field_ok`, with the negatives `take_decode_bad` (another field decoded) and `take_field_bad` (the map read after the removal) |
| | a callee's tuple result equated to an uninterpreted function's | `shapes TableTakes`: `swapped_ok`, `swapped_bad` |
| R14 a write through a returned element reference, read back (Rust) | | `rendered library_slice_get_mut_errors`: `increment` (and its negative `preserved`) |
| R15 a table's own invariant at an instance row | the rows | `shapes TableRows`: `row_instance_ok`, `row_nested_ok`, `row_tagged_ok` |
| | under a budget | the pin of `row_nested_ok`; `boundary CertifiedFieldProjections` |
| R16 behavioural invariants of a function-typed field, quantified over every state | the pack that proves them, and a caller that uses the no-abort and the result rule with no user-written precondition | `rendered mini_amm`: `make_capped`, `quote` |
| | what the pack must reject: a function that can abort, one whose result exceeds the bound | `rendered mini_amm`: `make_aborting`, `make_whole` |
| | a caller claiming more than the rules give | `rendered mini_amm`: `quote_strict` |
| | two named cases of the states the behavioural binder ranges over | `rendered mini_amm`: `make_fee_guarded` (positive: a state with `bps > 10000` is excluded) and `make_fee_unguarded` (negative: a function requiring a `Fee` is rejected, the empty state being admitted). The wider reading — the unit's installed ordinary data invariants and not its behavioural ones — rests on an inspection of the generator; this pair is its regression test |
| | the result rule written with `result_of` instead of `ensures_of` | open: `rendered mini_amm`: `make_bounded` proves it, `quote_bounded` cannot use it |
| | a pool holding a strategy, published, and the strategy invoked off a reference parameter | `rendered mini_amm`: `create`, and `quote_pool` (`&Pool`) and `quote_pool_mut` (`&mut Pool`, the Move Prover AMM's own shape) |
| | a **specification** global read of a resource that holds a function value | open: `rendered mini_amm`: `quote_stored` is never scheduled — `nativeSlot` builds a fallback for the whole resource carrier and needs it inhabitable |
| | the preservation of stored invariants after a call under `modifies_of *` | open: `rendered mini_amm`: `quote_stored_unbounded` reaches storage and the invocation, holds the strategy's two facts, and leaves the `stored_invariant` obligation |
| R17 a built-in map lemma's fact against the assertion that owes it: both read outside the map by the owner's typed view | a key read at a symbolic and at a ground position, at an owner whose values are of another width than its keys, and at one whose keys are a type parameter | `rendered map_key_default`: `key_at_rank`, `key_at_ground`, `key_at_rank_narrow`, `key_at_rank_generic` |
| | what the key lemma does not give: a fact of another key, a read outside the map (unspecified, not zero), and an integer-keyed law at keys that are no integers (refused, not synthesized) | `rendered map_key_default`: `key_at_other`, `key_at_outside`, `key_at_lt_generic` |
| | a value read of an updated and of a removed map, whose default is the values' and not the keys', and the value of a key without an entry | `rendered map_value_default`: `value_at_set`, `value_at_erase`, `value_at_missing` |
| R18 a law of the library applied by name in an authored proof: the size of a sequence map after an update of a key it holds and of one it does not, over the owner's own `add` | the two laws beside their parent `SpecMap.size_update_sequence`, and the cell of a key that may be held, which neither states | `rendered map_size_update`: `size_after_insert`, `size_after_override`, `size_after_unknown` |
| R19 the conditions of several kinds one site owes (not a plain-values regression: the kind-blind lookup predates it) | a mutable borrow whose write of memory owes the module's invariants and whose mutation end owes the resource's data invariant | `rendered condition_kinds`: `set_rate` (both hold), with `clear_rate` (the data invariant where the mutation ends) and `drop_interval` (the module invariant after the write of a mutable borrow) |
| | a declaration whose invariant is carried, where the mutation end owes nothing and the certifying cut owns it | `rendered condition_kinds`: `set_interval`, with `clear_interval` |
| | the module's invariants where a publication's write ends | `rendered condition_kinds`: `publish_flag` |
| | one site's two conditions read by their own markers, and a condition that is no predicate | `VerexIR/Tests/GivenWalker`: `markers_of_two_kinds`, `malformed_mutation_end` (`lake test` runs it; the internal malformed-obligation error is fail-closed) |
| R20 a match on a pair of a resource and an argument, followed by a closure invoke | the split reduces constructor readings (`withSome_some`, `withSome_none`); `process` and `plain` verify (the component-equation mechanism at the join is supported by source; individual join leaves were not traced) | `rendered split_pair_frame`: `process`, `plain`; `VerexIR/Tests/SplitOptionReadings` |
| R21 a join inside an inlined generic callee whose branched value has a closed, parameter-free invariant-bearing type | `verex_canonical_families` respells the value's carrier to the outer family while the `Flow` stays at `Skolems.instantiate`; `sealNode` peels the induced frame off the flow family when the descriptor is closed and parameter-free, and declines sharing when the descriptor mentions type parameters | `VerexSL/Tests/ContinuationJoins`: `select_meter`; `VerexIR/Tests/StageFamilyRecovery` |

Negative companions: where a class is represented by a positive rendering
(`static_step`, `static_fnonce`, `fnonce_forward`), the `_errors` fixtures
themselves are the gate's; what they reject is represented here by the three
rows above, and by nothing for a panicking method and for a tuple's wrong
component.

## Boundaries

| Boundary | Control |
|---|---|
| evidence that arrives before, and after, the read it belongs to | `VerexIR/Tests/VectorElementReads`, `VerexSL/Tests/ExecutableVectorReads`, `VerexSL/Tests/LeafOpenedFacts` |
| a fact of the same carrier at another type, frame or owner; a raw projection or an annotated term beside the application | `VerexSL/Tests/NominalFrameObservations`, `VerexIR/Tests/StoredBehaviorFrames` |
| an invalid value inside a chain of writes | `VerexSL/Tests/WrittenContainerHolds` |
| a declaration that is imported beside one that is local | `VerexIR/Tests/OwnBridgeDeclared`, `VerexSL/Tests/DenoteCacheImport` |
| the solver's rollback and the lifetime of its caches | `VerexIR/Tests/PreparationCache`, `VerexSL/Tests/DenoteCache` |
| a `verify` command beside its module's own verification; the Given route | `VerexSL/Tests/GivenOwnSelection`; `shapes Closures` and `shapes KnownClosures` verify in their module what a command does not establish |
| budgets of shared owners | `VerexIR/Tests/SharedBudgets`, `VerexSL/Tests/SharedPruning`, `VerexSL/Tests/SharedReferenceJoins` |
| a residual and the row of the callee it belongs to; a behavior fact that is not paid | `VerexIR/Tests/CallArgumentResidual`, `VerexIR/Tests/BehaviorDispatchResidual` |
| a dictionary's contract that is missing, or another function value's | `VerexSL/Tests/ForwardedMethodEvidence` |
| the generated bridge of an own invariant | `VerexSL/Tests/OwnBridgeGenerated`, `VerexSL/Tests/OwnBridgeLeaves` |
| the domain a call binds its result at | `VerexSL/Tests/CallResultDomains`, `VerexSL/Tests/CallResultHolds` |
| woven behaviors under a captured row's fact | `VerexSL/Tests/GuardedWovenReadings` |
| the theorems a rendered fixture advertises, beside its diagnostics | `VerexSL/Tests/MiniAmmPositives`, `VerexSL/Tests/MapDefaultPositives`, `VerexSL/Tests/ConditionKindPositives`, `VerexSL/Tests/ParkedGaps` |
| a literal map read whose default names a local family: a position inside it and a key with an entry evaluate, one outside and one without do not | `VerexSL/Tests/LiteralMaps` |
| a clause's concatenation with a singleton vector rewrites to append; the values of a nest read push steps as well as append steps | `VerexSL/Tests/ClauseSingletonConcat` |
| an element appended to a literal vector evaluates to the literal extended by that element | `VerexSL/Tests/LiteralVectorPush` |
| an owner's validity reads a fixed-width map value as its integer; a value of another type stays in its native representation | `VerexSL/Tests/CanonicalMapValidity` |
| the entries of a table written back, at each reader pair a key and a value can have | `VerexSL/Tests/WrittenBackRoundTrip`; the readers themselves in `VerexIR/Tests/TypedWriteback`, which `lake test` runs because its negatives are not silent |
| the element a successful read of a typed literal lookup yields, and the fallback it does not value | `VerexSL/Tests/LiteralVectorReads` |
| a range goal stated on a named result of a call | `VerexSL/Tests/RangeNamedResults` |
| a read of a written vector at a position the write leaves alone | `VerexIR/Tests/VectorWriteSizes` |

## Rendered fixtures

Each is the rendering `scripts/prover-smoke-renderings.sh` writes, unedited.
The generator refuses a fixture whose diagnostics the driver makes and a lone
elaboration cannot: a failing `no panic` or `no abort` report, a refused
function, a proof file beside the source, a verified namespace before the
last. Five further renderings are the reproducers of the section "Parked
gaps" below, which lists them instead of this table.

| Fixture | Stands for | Why no hand-written cell |
|---|---|---|
| `static_step` | a stateful callback under static dispatch of a user trait | the method contract and the receiver adapter are what the Rust exporter generates; `verex-ir` has no Rust profile to write them by hand |
| `static_fnonce` | the components of a tuple result of an `FnOnce` | the same |
| `fnonce_forward` | the three closure traits, their adapters, and forwarding through `ensures_of` of the trait method | the same |
| `library_slice_get_mut_errors` | a write through `get_mut`'s returned element reference, with its negative | the library's contracts are spliced companions of `verex-rust/lib` |
| `library_option_map` | `Option::map` with a closure | the same |
| `table_stored_resource` | a Move table as the Move frontend declares it: role bodies under `pragma intrinsic`, ghost memory, an open `modifies` | the hand-written cell `TableRows.publish_new_ok` has the mechanism; this has the declaration shape the frontend produces |
| `generic_function_literals_model` | `aborts_of` of a function literal in a specification, under the model | the model route is taken by the Move frontend's lowering of the literal |
| `map_key_default` | a built-in map lemma applied in a proof file, whose fact must be the fact the assertion owes: the keys' default for a key of its map, at owners whose keys and values read differently | the snapshot holds the proof file's items as items of the module (`PROOF_CONSUMERS` of `scripts/prover-smoke-rendering.py`, whose controls are `scripts/test_prover_smoke_rendering.py`), the one route by which an authored proof is elaborated by the libraries alone |
| `map_value_default` | the same for a value of the map, through the update and the removal lemma: the owner binds the two executable roles their facts are of | the same route; the roles are what a lemma of an updated or a removed map needs |
| `map_size_update` | a law of the library applied by name in an authored `lean` block: `SpecMap.size_update_sequence_{present,absent}` against the size an assertion owes after the owner's own `add` | the same route; the fact is of the map the executable role leaves, which only a frontend's rendering of that call produces |
| `condition_kinds` | the conditions of more than one kind that one expression owes: a mutable borrow of a resource owes the module's invariants where its write of memory ends and the resource's data invariant where its mutation ends, with a carried and an uncarried declaration and a publication, and a negative for each kind | the sites are the Move frontend's, which keys the write and the mutation of one borrow alike; a hand-written unit cannot state that collision |
| `mini_amm` | an automated market maker's rules as behavioural invariants of a function-typed field, quantified over every state: the pack, a caller that uses them, the pack's negatives, two named cases of the states the binder ranges over in a unit that also carries an ordinary data invariant, and the three storage observations (a reference parameter, a specification global read, the stored-invariant obligation after the call) | the wrapper is a Move tuple struct whose field is a function type and whose `spec` carries `modifies_of<self.0> *`; the positional field and the state binder are what the Move frontend writes, and `shapes KnownClosures` has no behavioural invariant |
| `split_pair_frame` | a match on a pair of a resource and an argument followed by a closure invoke (`calculator::process` of the benchmark): the split reduces constructor readings; `process` and `plain` verify (the component-equation mechanism at the join is supported by source; individual join leaves were not traced) | the pair-match lowering and inferred postcondition state labels (`..S1`, `S1..`) are what the Move frontend generates |

## Parked gaps

Five rendered fixtures hold a target the solver does not verify, kept as the
reproducer of a gap the user parked rather than repaired. A row here is a
recorded failure, not a regression class the suite guards: the baseline
records the diagnostics the gap produces today, and nothing in the suite
says the solver ought to behave that way. Each fixture also holds a control
that exercises the same construct without the gap, so the row distinguishes
the gap from a broken construct; `VerexSL/Tests/ParkedGaps` asserts that
each gap has no verified theorem and each control has one, and so fails and
names the fixture once a repair closes the gap. The fixture is then promoted:
its row leaves this section for the regression classes above, its target
becomes a control in that test, and its baseline is written again.

| Fixture: gap, control | Mechanism | What it stands for |
|---|---|---|
| `rendered split_reference_end`: `drain`, control `drain_automatic` | a discrepancy between the authored `compute` route and a prepared leaf: the prepared leaf's context holds the invariant fact the `data_invariant` obligation needs while under `compute` it is absent; the first missing correspondence and cause are unlocalized, and the fixture is no evidence for a match-split cause | the authored `compute` route discrepancy observed alongside `simple_map::add_all` (whose benchmark failure cause remains unlocalized); the same loop by the automatic route verifies |
| `rendered generic_join_ghost`: `settle_token`, control `pick_amount` | the unit declares a spec ghost, so a generic callee runs in its caller's walk: automatic verification fails at the join's induced-family premises (while the inspected authored `verex_decide` route verifies without forming the same join, narrowing the issue without establishing the first lost fact) | reproduces the automatic-route join failure in a unit with ghost state; 17 rows of `coin`, `fungible_asset` and `primary_fungible_store` in the framework sample share the excluding guard, with their individual causes not traced; a join of plain values owes nothing at the induced family and verifies |
| `rendered pure_spec_map_fact`: `v_len_set` and `v_set_param`, control `v_rank_nonneg` | at a spec-only map owner, the length-update lemma and the precondition equating a parameter with an updated map do not close these authored assertions; the parameter-map rank control verifies. The failing contexts include True hypotheses, but the first lost correspondence and cause remain unlocalized. | no benchmark or framework row is attributed to it: the mechanism is recorded by this reproducer alone |
| `rendered requires_of_disjunct`: `call_disjunct`, control `call_plain` | a callee's precondition `skip \|\| requires_of<f>(x)`, `f` a closure taking `&mut`, is not established at the call, and the target stops on the undecided leaf `requires_1_of_modify_at_call_1`; the precondition `requires_of<f>(x)` alone holds at the same kind of call | `big_ordered_map::test_verify_iter_modify` of the framework sample, whose leaf `requires_1_of_iter_modify` is undecided in the same way; that the two share a cause is not established |
| `rendered map_write_block`: `write_read`, control `write_read_rest` | a read of a map after one write, asserted under a block that takes one clause (`on assert 1 { }`): `decide` does not close it, and the leaf holds the write's equation `value_0 == spec_set(m, 1, 30)` beside the goal `spec_get(value_0, 1) == 30`; the same body under `on _ { }` verifies | `big_ordered_map::test_verify_upsert` of the framework sample, whose leaves after several map writes hold their equations unsubstituted; that the two share a cause is not established |

## Costs

Heartbeats of the interpreted elaboration, compared with
`scripts/prover-smoke-cost.pins` within 5% in either direction; a pinned
target that is not verified fails. A pin is not a check of what a target
proves.

| Fixture | Pinned | The kind of regression it is there for |
|---|---|---|
| `bench/lean/OrderedMap.lean` | `add`, `remove`, `borrow`, `lower_bound_loop`, `lookup_scenario`, `remove_edges_scenario`; two relative costs | the cost of the generic bodies and of the scenarios over them when a route of the solver changes |
| `Check/Closures/KnownSiteCost` | the quotient of one target in a module of nine closure sites and of three | a cost that grows with the closure sites of the unit |
| `Check/Examples/Quicksort` | `partition`, `quick_sort_range` | authored proofs over array lemmas |
| `Shapes/TableRows` | `row_instance_ok`, `row_nested_ok` | a table clause spelled so that each row costs again |
| `Shapes/Enums` | `enum_replaced_ok`, `vector_replaced_ok` | the call-contract stages of two map calls |
| `Cost/MapScaling` | `three`, `six` and their quotient | a cost that grows faster from three insertions to six than it does now; two sizes detect that and state no law of growth |
| `Rendered/mini_amm` | `quote` | the cost of using a function-typed field's behavioural invariants at a call: the no-abort rule and the result rule instantiated at the invocation's state |

The rendered fixtures' diagnostics are pinned by `#guard_rendering` and their
costs by the pins above; neither says that a positive cell still proves
anything. `boundary VerexSL/Tests/MiniAmmPositives` names the artifacts of
`rendered mini_amm` instead: each positive cell's `typedVerified` theorem is
present and each negative and open cell has none.

## Deferred

Check fixtures the script lists with every run and does not run, each with
its reason and what stands for it meanwhile. Nothing is deferred.

## Not covered

- Frontends, the driver, the command line, native executables, the Move
  Prover's lean tests, the Rust renderings and suite, compiled namespaces of
  a package (the gate runs `scripts/verify-compiled-namespaces.sh` for those).
- The rows marked "not covered" above.
- The open cells of the shape matrix.
- The three cells of `rendered mini_amm` marked open: the result rule
  written with `result_of` is proved at the pack and not read at a call; a
  specification global read of a resource holding a function value leaves its
  target unscheduled; and the same read removed, the call leaves the
  stored-invariant preservation obligation. All three are pinned by the
  fixture's baseline and by `boundary VerexSL/Tests/MiniAmmPositives`, so a
  tree that gains any of the capabilities fails the fixture. The third pins a
  residual-obligation dump, which is the most churn-prone text in the tier.
- Diagnostics are pinned where a negative names its clauses or a fixture has
  an `.exp`; their wording elsewhere is not.
