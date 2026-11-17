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

## Rendered fixtures

Each is the rendering `scripts/prover-smoke-renderings.sh` writes, unedited.
The generator refuses a fixture whose diagnostics the driver makes and a lone
elaboration cannot: a failing `no panic` or `no abort` report, a refused
function, a proof file beside the source, a verified namespace before the
last.

| Fixture | Stands for | Why no hand-written cell |
|---|---|---|
| `static_step` | a stateful callback under static dispatch of a user trait | the method contract and the receiver adapter are what the Rust exporter generates; `verex-ir` has no Rust profile to write them by hand |
| `static_fnonce` | the components of a tuple result of an `FnOnce` | the same |
| `fnonce_forward` | the three closure traits, their adapters, and forwarding through `ensures_of` of the trait method | the same |
| `library_slice_get_mut_errors` | a write through `get_mut`'s returned element reference, with its negative | the library's contracts are spliced companions of `verex-rust/lib` |
| `library_option_map` | `Option::map` with a closure | the same |
| `table_stored_resource` | a Move table as the Move frontend declares it: role bodies under `pragma intrinsic`, ghost memory, an open `modifies` | the hand-written cell `TableRows.publish_new_ok` has the mechanism; this has the declaration shape the frontend produces |
| `generic_function_literals_model` | `aborts_of` of a function literal in a specification, under the model | the model route is taken by the Move frontend's lowering of the literal |

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

## Deferred

Check fixtures the script lists with every run and does not run, each with
its reason and what stands for it meanwhile. Nothing is deferred.

## Not covered

- Frontends, the driver, the command line, native executables, the Move
  Prover's lean tests, the Rust renderings and suite, compiled namespaces of
  a package (the gate runs `scripts/verify-compiled-namespaces.sh` for those).
- The rows marked "not covered" above.
- The open cells of the shape matrix.
- Diagnostics are pinned where a negative names its clauses or a fixture has
  an `.exp`; their wording elsewhere is not.
