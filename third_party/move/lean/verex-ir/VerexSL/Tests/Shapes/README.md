# The shape matrix

One module per mechanism, one small function per type shape. A cell is a
positive (`*_ok`, a bare `verify`) and a negative (`*_bad`) whose diagnostic
is pinned: `expectUnestablished` names the clauses the report must leave
unestablished and no other, `expectCuts` the data-invariant cuts with their
boundary, `expectResidual` a failure that has no source clause to name. The
modules are roots of `VerexIRTests` and the first list of `scripts/prover-smoke.sh`.
Every negative and every open cell is a command of its own, so a module
reports each of its failures. `SupportControls` holds the controls of the
checks themselves: the uses of `expectOpen` and `expectResidual` that must be
refused.

Shapes: **a** scalar; **b** struct with a data invariant / without one;
**c** enum with a per-variant invariant clause; **d** struct over an
invariant-bearing struct / over a vector of them; **e** type parameter;
**f** generic struct at an instance; **g** tuple; **h** `&mut`; **i** function
value.

| Module | Mechanism | Shapes with a cell |
|---|---|---|
| `Construct` | construct and return a value | a b c d e f g i, a generic struct whose invariant bounds its vector |
| `Mutate` | write through a `&mut`, end the mutation | a b c d e f g h i |
| `CallArguments` | pass to a contract call | a b c d e f g h i, two invariant-bearing arguments |
| `CallResults` | receive a call's result, use its invariant | a b c d e f g i, a result equated to a specification function |
| `Generics` | a generic opaque callee at the caller's parameter and at an instance | a b c d e f g h i, recursive specification functions |
| `OpaqueGenerics` | an opaque specification function under a generic native's type parameters | b a e f, a nominal argument the caller constructs, a function at a type argument alone with a result of type `T` |
| `Storage` | publish, keyed read, take, `exists` | b c d e f |
| `StorageBorrows` | write a resource in place; read memory after a write, a publish, a call | b, at the same key, another key, another resource |
| `Vectors` | push, pop, swap, swap-remove, element read and write, a removal through a borrow of a generic holder's field, a generic enum built from a popped element | a b c e f |
| `Tables` | intrinsic map read, insert, remove, mutable borrow, presence, a table held by a resource | a b e f |
| `TableRows` | a table's rows at resolved type arguments; a table behind an owner | generic struct row, nested table, opaque invariant at `T`, generic owner, published fresh table |
| `TableTakes` | an entry taken out of a stored map, unpacked, handed to opaque calls; a tuple result equated to an uninterpreted function's | a struct value over a generic key, at the caller's `T` |
| `Enums` | match, variant by branch, shared field, optional map results | c, in the enum and the vector representation |
| `Closures` | function values: unseen target, named function, capture, pass-through, a generic higher-order function of one and of two arguments | i, result with and without an invariant, at `R` |
| `KnownClosures` | closures a proof sees; function values in storage | i: capture, generic target, struct field, vector element, two arguments, enum resource |
| `MethodRejections` | a trait method passed as a dictionary: what a caller's clause and a method's body must not get away with | i, with a `&mut` receiver |
| `Control` | join of two branches, loop | b c e f h |

Shapes with no cell of their own: `&mut` is borrowed, never constructed or
returned as a fresh result; a tuple, a reference and a function value are not
resource types.

## Open cells

A cell this tree does not establish stays in its module under `expectOpen`:
the verification must reject it, the FIRST diagnostic of the rejection must
begin with the text recorded here, and every other diagnostic must be of the
categories a verification failure of the cell reports (an unestablished
clause, a data-invariant cut, a continuation or precondition of a call, a
residual obligation). The check does not pin the whole list of diagnostics
and proves no cause. The module turns red the moment the cell is
established, where it moves up to the positive controls and gets its
negative. An open cell is a sentinel of a missing capability, not coverage:
a green module does not say its open cells verify. The list:

| Cell | First diagnostic | Cause, as far as it is known |
|---|---|---|
| `Mutate.element_write_ok` | the data invariant `invariant value <= 100` does not hold where a mutable parameter's loan ends, at the function's exit | a lookup into the written vector behind a struct field is not reduced at the written index |
| `Mutate.fn_field_write_ok` | the specification clause `f : Fn(u64) -> Percent has Copy, Drop` is not established | a function value built at a site carries no validity fact of its own |
| `CallArguments.vector_ok` | the specification clause `ensures result <= 100` is not established | a vector's fact is not opened at the element the goal reads, behind the callee's `ensures` |
| `CallResults.vector_ok` | the specification clause `ensures 0 < result.items.length ==> result.items[0].value <= 100` is not established | the same, for a returned vector |
| `CallResults.fn_ok` | the continuation after call 1 of `make_maker` is not established | the invocation of a function value that is a call result is left as a raw `wp` |
| `Generics.fn_ok` | the precondition of `apply` is not established | three facts the callee's contract owes at `T := Percent`: `ScalarAt` of the row `T`, false at a declared type (a limit of the rule for an unseen invocation and of the contract generated from the signature, not a need of every higher-order function); the function literal's frame at the frame the type arguments induce; the validity of its results at that frame. For the frame, the step moves the fact to the base frame and then does not recognize the literal in the moved statement; for the results, the step that reads the target's theorem does not move the fact to the base frame first. The cell has no negative while it is open |
| `Tables.borrow_mut_ok` | the specification clause `ensures get(t, k).value == amount` is not established | the equation of a map written through a mutable borrow is stated of another form of the map than the read |
| `Tables.stored_write_ok` | the specification clause `ensures get(global<Registry>(addr).slots, k).value == amount` is not established | the same, for a table held by a resource |
| `Closures.unseen_ok` | verification failed with a residual obligation | the result invariant of a function value a proof cannot see is not available to the caller |
| `Closures.field_ok` | verification failed with a residual obligation | the invocation of a function value held in a struct field is left as a raw `wp` |
| `Closures.field_behavior_ok` | verification failed with a residual obligation | the same, for its behaviour |
| `Closures.stored_ok` | verification failed with a residual obligation | the same, for a function value held by a resource |

Three further facts a cell does not pin: a mutable reference derived from a
parameter and returned to the caller carries no invariant obligation at the
function's exit; a contract that reads `ensures_of` of a function-typed
parameter is established where its module is elaborated and not through a
later `verify` command of a module under `pragma verify = false` (the
positives of `Closures` and the stored function values of `KnownClosures` are
therefore verified in their module); and `KnownClosures` does not verify
`apply_state`, which invokes a function value out of a matched variant.
