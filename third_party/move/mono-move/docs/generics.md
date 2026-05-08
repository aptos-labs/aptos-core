# Generics in MonoMove

## Usages

Runtime types appear as:

- Keys for storage.
- Keys for caches of monomorphized code.
- Inputs to layout computation.
- Typed-slot reuse in destacking (slots with same types can be reused if their lifetimes do not overlap).

What we want from the representation:

- Cheap equality, hash, clone: amortized O(1).
- Substitution that does not pollute memory and is cheap to construct.

The monomorphized function code and layout caches are concurrent and shared across worker threads, so the representation must support cheap concurrent lookup without driving unbounded memory growth.

## Two options

1. All types are canonicalized (a single shared DAG).
   Equality and hash are pointer-cheap, but creation goes through a global structure.
   Substitution is worse than creation, because it has to re-canonicalize each rewritten subterm.

2. All types are atoms.
   Creation and substitution are cheap.
   Equality and hash are structural, paying a tree walk on every comparison.

Either side alone is bad.
The DAG churns memory at runtime, the atom form makes the hot cache paths, equality, and copy expensive.

## Proposal: Hybrid mode

The static part of the program is canonicalized into a global structure.
The dynamic part (runtime substitutions) stays atom-like, layered on top.
A reconciliation step bridges the two when caches need to compare types logically.

The rest of the document develops this in four observations, then describes how operations work on the resulting representation.

## Observation 1: intern all signature tokens from the file format

The set of types and type lists that appear in source bytecode is bounded and known at module publication.
We intern all of them once.
Pointer equality on an interned type then implies structural equality.

We intern both closed types and open templates.
`A<TP{0}>`, `B<TP{0}>`, and `A<u8>` are all separate interned types if all three appear in source.
Templates use de Bruijn indices for `TypeParam` leaves, so they are context-relative, but interning treats them as syntactic terms.
Conceptually this is the same as interning canonicalized formulas: `λx. Pair<x, u8>` is one interned term even though `x` is bound.

Its size of interner's memory is bounded by the program's source, and all signatures are bounded in size.
It can grow at runtime only as cache grows = so it is the same as what exists on-chain.

## Observation 2: cannot intern all type substitutions eagerly

Eagerly canonicalizing every runtime-substituted type is not viable:

- The set of fully-applied generic instantiations reachable at runtime is unbounded relative to source (check this claim: source can upgrade, can we create unboundedness this way?).
  A transaction can build `f<g<h<...>>>` to arbitrary depth through generic call chains.
  Inserting all reached shapes into the immortal interner grows it across transactions, and an adversary can drive that growth.
- We intern eagerly even if type is not used.
  For example, if fucntion uses some generic instantiation, it does not need to be constructed unless actually used.

We need a representation that defers canonicalization to the moment of observation, and only pays for shapes we actually have to compare.

## Observation 3: substitution is composition of applications

A runtime type at any call site is a chain of applications of source-interned substitutions to a source-interned template:

```
σ_n . σ_{n-1} . ... . σ_1 . template
```

Each `σ_k` is itself an `InternedTypeList` from source, drawn from the call site's bytecode type-argument list.
The `template` is also from source.
So a runtime-substituted type is a stack of pointers into the global structure:

```rust
pub struct SubstType {
    pub template:  InternedType, // or InternedTypeList
    pub env_stack: Vec<[InternedTypeList]>,
}
```

Construction is O(1) and uses little memory.

### Stack semantics

`TypeParam{i}` in `template` resolves through `env_stack.last()`.
`TypeParam{i}` in `env_stack[k]` resolves through `env_stack[k-1]`.
The deepest layer's entries are closed source `InternedType`s.

### Example

Source has `T_A = A<TP{0}>`, `T_B = B<TP{0}>`, `U8`, and `T_A_U8 = A<u8>`, but not `B<A<u8>>`.
The runtime type `B<A<u8>>` is one of:

```
template  = T_B,  env_stack = [ [T_A_U8] ]
template  = T_B,  env_stack = [ [U8], [T_A] ]
```

Both denote the same logical type.
The first is shorter because more of the structure already exists in source.

## Observation 4: stacks compress

Threading a generic parameter through several callers without specialization (`f<T>` calls `g<T>` calls `h<T>`) produces an env stack of identity layers.
Those layers collapse — they carry no new substitution.

The compressed form is the stack with no further reduction available.
Reduction is the explicit mechanism that brings any stack to its compressed form against the source interner.
Each step probes the source interner for the result of applying one layer; if found, the layer collapses; if not, reduction stops at a partial fixed point.
The probe is a lookup, never an insert. No new state lands in the immortal interner.

## Operations on types

### Copy and clone

Cheap.
A `SubstType` is a small array of pointers, almost always one `memcpy`.
The size of `SubstType` is bounded by signature token depth or call stack depth (each call can instantiate more types).
So this is at most 1024 (current VM limit).

### Cache keys

Cheap to use as keys.
The risk is logical: two different stacks can denote the same logical type, so one logical type can produce multiple cache entries.

For the layout / size cache, this only means we recompute the same layout twice.
For the monomorphization cache, this is more costly: the same code is compiled twice and held twice in memory.
This is better ot be avoided.

So the central question is: how do we detect logical equality without paying full structural canonicalization on every operation?

### Equality and hash

The cheap version is pointer-composite, no tree walk:

- Hash: pointer hash of `template`, then pointer hash of each `InternedTypeList` in `env_stack`.
- Eq: pointer compare on `template`, length compare on `env_stack`, pointer compare per layer.

This catches structurally identical `SubstType`s.
It misses two `SubstType`s that denote the same logical type via different stacks. For example,

```
S1 = (T_C, [[U8], [B<TP{0}>], [A<TP{0}>]])
S2 = (T_C, [[B<u8>],          [A<TP{0}>]])
```

are logically equal but pointer-distinct.
Hash-consing `SubstType` itself would dedup exact duplicates but would not collapse `S1` and `S2`
So pointer-composite hash and equality alone are not enough for the monomorphization cache.

### Substitution and creation

O(1).
A new `SubstType` pushes one `InternedTypeList` onto the env stack of an existing template.
No allocation beyond the slot.

### Traversal

Required for layout and reduction.
Both visit the template tree once, resolving `TypeParam` leaves through the env stack as they go.
Same as traversing interned types if implemented correctly (to avoid repeated substitutions and keep traversal O(|type DAG|)).

## Union-find for logical equality

One possible conceptual answer is union-find (UF).
We can lazily merge logically-equal types into the same equivalence class.
Each class has one representative.
The representative is either a `SubstType` or, if the class has reduced to a closed source-interned shape, an `InternedType`.
`find(s)` returns the best representative of `s`'s class.

UF then becomes the indirection layer:

- Cache keys are roots, not arbitrary `SubstType`s. Two routes to the same logical type land on the same key.
- Equality short-circuits on same-root.
- The only structural work is the lazy union step on first comparison.

Then the question is how to implement such a UF structure.

## Option 1: separate UF context, immutable `SubstType`

We do not mutate the type itself.
Instead, a UF context maps types to their equivalence classes, leaving `SubstType` immutable.

### Complexity of type operations

| Operation             | Cost                                                         |
|-----------------------|--------------------------------------------------------------|
| Construct `SubstType` | O(1), no allocation                                          |
| Copy / clone          | one memcpy of a small pointer array                          |
| Hash (cheap)          | few pointer hashes                                           |
| Equality (cheap)      | few pointer compares                                         |
| Equality through UF   | one `find`, then root compare                                |
| Substitution          | O(1), push one layer                                         |
| Traversal             | one walk of the template, resolving leaves through the stack |

Reduction depth is bounded by the call chain's generic nesting, which is bounded by the VM's existing depth limits.

### Challenges

For runtime equality checks inside a single thread (e.g. type checks on stack values), a thread-local UF is enough.
UF ids are local, but that does not matter: the comparisons stay within the thread.

However, this does not work for concurrent caches that use  `SubstType` as keys.
For example, monomorphzied code cache.
A thread-local UF cannot make two threads agree on a representative for the same logical type — their UF ids are not comparable.
Caching keyed on thread-local roots would not converge across threads, and threads racing on the same logical type would compile and store independently.

To solve the issue, there are a few options:

- **Accept duplication.**
  Use the cheap pointer-composite key.
  Two routes to the same logical type produce two cache entries. 
  Correctness is intact. The cost is bounded duplicate compilation and bounded duplicate compiled code in memory.
  For closed-form instantiations whose canonical list appears in source, reduction collapses both routes to the same pointer and there is no duplication.
  Duplication is confined to open forms whose canonical list is not in source.
  Alternatively, deduplicate by values.
  For example, this way same monomorphized code can be shared by different `SubstType`s.

- **Concurrent UF.**
  Solves cross-thread dedup, but a correct lock-free UF (atomic parent slots, CAS, union-by-rank with linearization) is non-trivial.
  A naive shared parent table is broken under concurrent `union` — two threads choosing opposite parent directions can create cycles and infinite `find` loops.

## Option 2: mutable `SubstType`

Also seems problematic as atomic parent pointer needs to be maintained.
So conceptually every `SubstType` has to be `Arc`ed.
