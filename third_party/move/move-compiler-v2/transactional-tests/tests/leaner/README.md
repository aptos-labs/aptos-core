# Leaner transactional coverage

Each `.lean` file is compiled through XIR, loaded into the Move model as
stackless bytecode, processed by compiler v2, emitted as Move bytecode, and run
by the production VM harness. The dedicated `leaner` test configuration runs
this pipeline once with compiler-v2's default experiments; Leaner sources are
excluded from the generic optimization configuration matrix.

Transactional commands use the Lean-comment-compatible `--#` prefix. The local
Lake wrapper depends on the Lean project providing `LeanerMove`, so these files
also elaborate in the Lean language server without any preprocessing.

Each file imports `LeanerMove` and declares exactly one module with
`leaner module 0xADDR::Name where ...`; every declaration of that module
(structs, enums, functions, and `spec` blocks) is compiled and published. A
`spec f where ...` block verifies `f` against its contract, and a failing
contract rejects the file.

| File | Coverage |
| --- | --- |
| `basic.lean` | Private-function invocation, explicit abort, and `u64` arguments |
| `abilities.lean` | Exact `Copy`, `Drop`, `Store`, and `Key` declarations (`has`) for structs, enums, and generics |
| `arithmetic.lean` | Returned `u64` values, locals, add/subtract/multiply/divide/modulo, and arithmetic failures |
| `addresses.lean` | Registered address aliases, aliased module identities, literal address values, address equality, and calls at a non-zero module address |
| `control_flow.lean` | Returned branch values, `<`, `<=`, equality, Boolean connectives, nested branches, join points, and a counting loop |
| `calls.lean` | Returned values from pure/effectful calls, bound results, nested calls, direct recursion, and mutual recursion |
| `tail_recursion.lean` | Large-input loops that replace tail calls (with early `return`, a swap through a temporary, and a final recursive step) and preserved non-tail recursion |
| `loops.lean` | `while` and `loop`, `break`/`continue`, labeled loops, and `return` inside a loop |
| `vectors.lean` | Vector literals, length/get/set, and immutable/mutable element borrows |
| `vector_operations.lean` | Empty/push, nested and Boolean vectors, insert/remove through a vector reference with stable shifting, edge updates, freeze, post-write borrowing, and bounds failures |
| `enums.lean` | Nullary, unary, and multi-field variants plus exhaustive matching |
| `enum_patterns.lean` | Nested matches over enum payloads, multiple payloads, and wildcard fallbacks |
| `enum_payloads.lean` | Duplicate and positional field names, single variants, vector payloads, vectors of enums, wildcards, and calls carrying enums |
| `generics.lean` | True generic structs, resources, enums, functions, nested instantiated calls, vectors, and distinct storage identities for two instantiations through compiler v2 and the VM |
| `closures.lean` | Function values with leading and trailing captures, a generic target, closures returned, held in a struct field, passed to a higher-order function, aborting in their target, and stored in a resource (`Store` through a public target) |
| `ordered_map.lean` | Generic sorted-vector map, binary search as a loop, key ordering through `std::cmp`, implicit freezing, borrowed lookup, vector insertion/removal, Boolean keys, and duplicate/missing-key aborts on MoveVM |
| `reject_recursive_enum.lean` | Recursive enum declarations are rejected explicitly |
| `reject_continue_outside_loop.lean` | `continue` outside a loop is rejected at the source boundary |
| `reject_undeclared_call.lean` | Calls to host Lean definitions or other undeclared functions are rejected at the source boundary |
| `reject_verification.lean` | A `spec` whose `ensures` the implementation does not establish rejects the file |
| `reject_empty_enum.lean` | An enum without variants must be rejected (currently lowered to a zero-sized struct and caught only by the bytecode verifier) |
| `references.lean` | Private resource functions, immutable/mutable nested field borrows, reads, writes, propagated `acquires`, and missing-global failures |
| `borrow_checker/` | Poison-aware source acceptance/rejection, exact Leaner diagnostics, compiler-v2 comparison failures, production-verifier comparison failures, and successful VM executions |
| `reject_recursive_type.lean` | Recursive structs are rejected while recursive functions remain supported |
| `reject_recursive_generic_type.lean` | Mutual recursion through generic struct instantiations is rejected |
| `reject_invalid_ability.lean` | A declared `has` ability is rejected when a field lacks it |
| `reject_unsupported_type.lean` | A LeanerLang type without a Move runtime representation (`Nat`) is rejected at the XIR boundary |

The reference test prints bytecode so the baseline also checks the generated
resource and reference instructions. The generic test publishes, queries, and
moves two instantiations of the same generic resource at one address, checking
that production bytecode preserves their distinct storage identities.

Successful computations are checked through ordinary function return values.
`abort` is reserved for tests which intentionally exercise abort behavior.
