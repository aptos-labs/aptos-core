# Leaner transactional coverage

Each `.lean` file is compiled through XIR, loaded into the Move model as
stackless bytecode, processed by compiler v2, emitted as Move bytecode, and run
by the production VM harness. The dedicated `leaner` test configuration runs
this pipeline once with compiler-v2's default experiments; Leaner sources are
excluded from the generic optimization configuration matrix.

Transactional commands use the Lean-comment-compatible `--#` prefix. The local
Lake wrapper depends on the main Lean project, so these files also elaborate in
the Lean language server without any preprocessing.

Deployable sources can use `module Module where ...`, which combines the
namespace and export and treats ordinary `def`s as private Move functions.
Compilation is deferred until end of input so the entire block is included.

| File | Coverage |
| --- | --- |
| `basic.lean` | Private-function invocation, explicit abort, and `u64` arguments |
| `abilities.lean` | Exact `Copy`, `Drop`, `Store`, and `Key` deriving for structures, enums, and generics |
| `arithmetic.lean` | Returned `u64` values, locals, add/subtract/multiply/divide/modulo, and arithmetic failures |
| `addresses.lean` | Registered address aliases, aliased module identities, literal address values, address equality, and calls at a non-zero module address |
| `control_flow.lean` | Returned branch values, `<`, `<=`, equality, nested branches, join points, and tail recursion |
| `calls.lean` | Returned values from pure/effectful calls, bound results, nested calls, direct recursion, and mutual recursion |
| `tail_recursion.lean` | Stack-safe pure/effectful tail recursion, parallel loop-parameter updates, and preserved non-tail recursion |
| `vectors.lean` | Vector literals, length/get/set, and immutable/mutable element borrows |
| `vector_operations.lean` | Empty/push, nested and Boolean vectors, native insert/remove with stable shifting, edge updates, freeze, post-write borrowing, and bounds failures |
| `enums.lean` | Nullary, unary, and multi-field variants plus exhaustive matching |
| `enum_patterns.lean` | Nested constructor patterns, multiple nested payloads, wildcards, and inner-pattern fallthrough |
| `enum_payloads.lean` | Duplicate and positional field names, single variants, vector payloads, vectors of enums, wildcards, and calls carrying enums |
| `generics.lean` | True generic structs, resources, enums, functions, nested instantiated calls, vectors, and distinct storage identities for two instantiations through compiler v2 and the VM |
| `ordered_map.lean` | Generic sorted-vector map, binary search, implicit freezing, borrowed lookup, native vector insertion/removal, Boolean keys, ordering, and duplicate/missing-key aborts on MoveVM |
| `reject_non_tail_continue.lean` | `continue` rejects a self-call used outside tail position |
| `reject_non_self_continue.lean` | `continue` rejects calls which do not target the current function |
| `reject_indexed_enum.lean` | Indexed enum declarations are rejected explicitly |
| `reject_recursive_enum.lean` | Recursive enum declarations are rejected explicitly |
| `reject_empty_enum.lean` | Empty enum declarations are rejected before XIR emission |
| `references.lean` | Private resource functions, immutable/mutable nested field borrows, reads, writes, propagated `acquires`, and missing-global failures |
| `borrow_checker/` | Poison-aware source acceptance/rejection, exact Leaner diagnostics, compiler-v2 comparison failures, production-verifier comparison failures, and successful VM executions |
| `reject_unselected_call.lean` | A Move-attributed helper must be selected in the same module request |
| `reject_ordinary_call.lean` | Calls to arbitrary Lean functions are rejected at the source boundary |
| `reject_recursive_type.lean` | Recursive data types are rejected while recursive functions remain supported |
| `reject_recursive_generic_type.lean` | Indirect recursion through generic type instantiations is rejected |
| `reject_invalid_ability.lean` | A derived ability is rejected when a field lacks its required ability |
| `reject_unsupported_type.lean` | Reserved but not-yet-enabled source types are rejected explicitly |

The reference test prints bytecode so the baseline also checks the generated
resource and reference instructions. The generic test publishes, queries, and
moves two instantiations of the same generic resource at one address, checking
that production bytecode preserves their distinct storage identities.

Successful computations are checked through ordinary function return values.
`abort` is reserved for tests which intentionally exercise abort behavior.
