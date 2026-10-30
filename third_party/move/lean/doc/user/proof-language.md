# The proof language

Verex verifies a function automatically within a budget: cheap, bounded
deciders over its obligations, no SMT, every proof checked by the Lean
kernel. An obligation the automation does not close is proved by hand — by a
person or an agent — in the proof file beside the Move source. The proof
language is what those proofs are written in.

This chapter is readable on its own; the Move side of its examples needs only
Move specification syntax. The design, lowering, and rationale are in
[`../plan/proof-language.md`](../plan/proof-language.md), which also records
what has landed and what is still in flight. The examples are the proof
files of the framework's `ordered_map` and of the end-to-end fixtures under
`verex-e2e-tests/VerexE2ETests/SourceVerify`, shortened where noted.

## Obligations

Verifying `f` means discharging its *obligations*:

- each `ensures`, `aborts_if`, and `aborts_with` clause of its spec, by index
  in the block (`ensures 2`);
- each `requires` clause of every callee, owed at each call
  (`requires 1 of iter_next at call 2` is the first precondition of
  `iter_next`, owed at its second call in the body; `requires at invoke 1`
  the precondition of the first invocation of a function value);
- each loop invariant, on entry to the loop and at the start of every
  iteration (`invariant 2 of loop 1 step`; `invariant 2 step` where the
  function has one loop);
- each in-body `spec assert` (`assert 1`);
- the data invariants of values the function builds, and the module and
  stored invariants it owes (`data invariant`, `module invariant`, `stored
  invariant`); a lemma application's premise (`lemma requires`), an
  unrolled loop's bound (`unroll`), and a ghost field's domain (`ghost
  domain`).

An obligation is keyed by what it is — the clause and the site that raises it
— never by a position in some generated term. Sites are counted per kind in
source order of the body, so a key changes only when the Move source does. A
clause owed on several paths through the body is one obligation, proved
under the disjunction of those paths; a proof can also take paths apart
individually ([Selectors](#points-and-selectors)).

## The proof file

The proofs of `foo.move` live in `foo.proof.lean` beside it:

```lean
import VerexMove.Proofs

verex proofs for "foo.move"
verex proofs where

verify f by
  …
```

A `verify f by …` item owns `f`'s whole goal: no automatic attempt runs
first, and the block proves every obligation. Items may be interleaved with
`open` declarations and `lemma` items (below). `verex proofs where` verifies
the module — its theorems and lemmas, then its targets, callees first;
functions no item names get the automatic attempt.

`pragma verify = manual` in `f`'s spec block states that `f` is established
by a proof file: it is not attempted automatically, and a missing item is an
error. When the automatic verification of a function fails, the message names
the obligations left open, states them in specification form, and prints the
proof it asks for as a template in this language, ready to fill in.

Expressions in a proof are VerexSL specification expressions, the dialect
of chapter 1: `∀ (i : Int), …`, `spec_len(m)`, `v[i]`, `old(m)`, `result`,
`it is End`, `it.index`. Move's own spelling of a quantifier (`forall i:
u64: …`) is not accepted here.

## A proof

`ordered_map::test_verify_drain_symbolic` removes the first entry of a map
until it is empty; its loop invariants say the enumeration shifts down by
one per removal:

```move
while ({
    spec {
        invariant count + spec_len(m) == spec_len(old(m));
        invariant forall i in 0..spec_len(m):
            spec_key_at(m, i) == spec_key_at(old(m), i + count);
        invariant forall i in 0..spec_len(m):
            spec_get(m, spec_key_at(m, i)) == spec_get(old(m), spec_key_at(m, i));
    };
    !m.is_empty()
}) {
    let (_k, _v) = m.pop_front();
    count += 1;
};
```

The proof, from `ordered_map.proof.lean`, with the third invariant's block
elided (it follows the second's pattern):

```text
verify test_verify_drain_symbolic by
  on invariant 1 step {
    apply lemma_contains_key_key_at(m, 0)
    assert spec_len(m') == spec_len(m) - 1 by apply lemma_len_remove(m, spec_key_at(m, 0))
  }
  on invariant 2 step {
    let k0 := spec_key_at(m, 0)
    apply lemma_contains_key_key_at(m, 0)
    assert spec_len(m') == spec_len(m) - 1 by apply lemma_len_remove(m, k0)
    apply lemma_rank_key_at(m, 0)
    assert spec_key_at(m', i) == spec_key_at(m, i + 1) by apply lemma_key_at_remove(m, k0, i)
    assert spec_key_at(m, i + 1) == spec_key_at(old(m), i + 1 + count)
    assert i + 1 + count == i + (count + 1) by arith
  }
  on invariant 3 step { … }

  on aborts_if 1, ensures 1, ensures 2, invariant 2 entry { }
```

The proof is the mathematical argument and nothing else: it names the lemma
each fact comes from, states the two arithmetic identities, and leaves the
rest to the decider. `m'` is the map after the iteration's `pop_front`, `i`
the invariant's quantified position. An obligation named by no `on` gets the
cheap deciders alone; `on _ { }` covers the remainder explicitly.

The three blocks repeat the same facts, because each `on` sees one
obligation. Facts that every invariant of the step needs can be stated once
at a point after the body's call instead, `after call 1 of pop_front { … }`.
`at loop 1 step` is the start of an iteration, before the body: `m` is the
map the iteration starts with, and `m'` is not yet bound there.

## Steps

| Step | Meaning |
|---|---|
| `let x := e` | bind `e` for the following steps |
| `assert e by c` | owe `e`, proved by closer `c` (`decide` without `by`), then use it as a fact |
| `assert e by contradiction { … }` | run the block under `!e`; closes when `decide` finds `false` |
| `assume e` | use `e` without proof, for this obligation only; enters the assumption ledger, with a warning. Allowed inside an `on` block only |
| `apply φ, …` | apply lemmas or recursive spec functions at the stated arguments: owe their premises (by `decide`), gain their conclusions; `apply c ==> L(a)` guards both by `c` |
| `split e` | one case per value of a specification expression: both sides of a Boolean, each variant of an enum or option (payload fields under their Move names), each closure site of a function value plus `foreign` |
| `split i in a..b` | one case per value of an integer the context bounds by literals |
| `let V { f, … } := e` | destructure a value known to be of variant `V`; owes `e is V`, creates no cases |
| `calc(e₁ op e₂ …)` | a chain of steps, each proved by the closer, conclusion gained |
| `forall x : T where g { … }` | run the block under `x` and `g`; gain `∀ x, g ==> (facts asserted)` |
| `witness e, …` | supply the witnesses of an existential obligation or `assert`; its body is then owed |
| `if e { A } else { B }` | gain `e ==> A` and `!e ==> B` without splitting the proof |
| `on k, … [when d, …] { … }` | prove the named obligations, on the paths the selector takes |
| `at p [when d, …] { … }` / `after s { … }` | place facts at a program point, owed once, inherited by every obligation after it |
| `lean => tac` | run Lean tactics on the goals at this step |

The steps after a `split` run in every case. From `proof_steps.proof.lean`,
whose specs quantify so that the blocks have something to do:

```text
verify bump by                       -- ensures ∀ (i : u64), i <= x ==> i + 1 <= result
  on ensures 2 {
    forall i : Int where i <= x {
      calc(i + 1 <= x + 1 == 1 + x) by arith
    }
  }
  on _ { }

verify clamp by                      -- ensures ∀ (i : u64), i > 10 ==> result < i
  on ensures 1 {
    if x > 10 {
      assert x >= 11
    } else {
      calc(x <= 10 < 11)
    }
  }

verify pick by                       -- ensures ∃ (y : u64), y < result
  on ensures 1 {
    witness x
  }
  on _ { }

verify same_flag by                  -- callee requires ∃ (y : T; k : u64), y == _x && 2 * k == n
  on requires 1 of same {
    witness true, m + 1
  }
  on _ { }
```

A call's results are named in a proof by the callee and the call: in
`twice`, which calls `inc` twice, the first result is `old_inc_result` and
the second `inc_result`.

A `split` of an iterator takes its variants apart; from
`test_verify_iter_walk_mut_symbolic`, which writes `7` at every position of
a map through its iterator:

```text
verify test_verify_iter_walk_mut_symbolic by
  on invariant 6 step {
    split it
    split iter_next_result
    let k := spec_key_at(m, count)
    apply lemma_contains_key_key_at(m, count)
    apply lemma_len_set(m, k, 7)
    apply lemma_key_at_set_present(m, k, 7, i)
    apply lemma_get_set(m, k, 7, spec_key_at(m, i))
    apply lemma_key_at_eq_iff(m, i, count)
    split i == count
  }
  on invariant 4 entry, invariant 5 entry {
    split internal_new_begin_iter_result
  }
  on _ { }
```

The last `split` separates the position just written from the earlier
ones; `decide` closes each case from the two lemmas before it.

### Points and selectors

Points are `at entry`, `at exit`, `at loop k entry|step`, and `after s`, the
continuation of a site `s` — `call k (of g)`, `invoke k`, `abort k`, `op k`
(the `k`-th call, invocation, `abort`/`assert!`, or aborting operation, by
source order). Facts at a point hold on every path through it and are owed
once, so a fact several obligations need is stated once, not per clause;
prefer points over `on` wherever a fact is not clause-specific. From the
`proofs` fixture, where the `ensures` is the expansion of a square and the
abort conditions need its sign:

```text
verify square_of_sum by              -- fun square_of_sum(a, b) := (a + b) * (a + b)
  at entry {
    assert (a + b) * (a + b) == a * a + 2 * a * b + b * b by ring
    apply lemma_mul_nonneg(a + b, a + b)
  }
```

An `assert` at a point is an obligation of its own, as an `assert` of a Move
proof block is, keyed by the point: `assert 1 of at entry`, `assert 2 of after
call 1 of g` (the `k`-th assert at that point). Without a closer it is routed
like any obligation, to the automatic solver or an `on assert k of p { … }`
block; with one, `assert e by c` is shorthand for that block. Either way the
fact holds after the point.

Selectors take paths apart by the source decisions on them: `if 3 then`,
`match 2 Some`, `loop 1 exits`, `invoke 1 is g`, `aborts at call 1 of
add_shares`. A path goes to the first `on` in text order whose key and
selector match it, so a block for selected paths precedes the general block
for the same clause. `pool_u64::buy_in` aborts either in its own addition
or inside `add_shares`:

```text
verify buy_in by
  on aborts_if 1 when aborts at op 3 {
    assert old(self).total_coins + coins_amount > MAX_U64 by arith
  }
  on aborts_if 1 when aborts at call 1 of add_shares {
    split spec_contains_key(old(self).shares, shareholder)
  }
```

A selector that matches no path is an error listing the decisions of the
key's paths.

## Closers

| Closer | Decides |
|---|---|
| `decide` | the default: a fact stated by a hypothesis (or its disjunct, an equation either way round), a contradiction in the facts in scope, an instance of a quantified hypothesis, reflexivity, or linear arithmetic over the facts in scope; a hypothesis `p ==> q` or `p <==> q` gives `q` where `p` is stated. Bounded; no search |
| `arith` | linear arithmetic with the stated bounds, no case splits |
| `eval` | closed maps, literal vectors, ground arithmetic |
| `ring` | polynomial identities over the integers, `e₁ == e₂` |
| `congr` | equality through function applications and uninterpreted terms, with linear arithmetic; no open `if`, no existential |
| `solve` | the automatic leaf under its cap — an explicit request for automation, charged as automatic work |
| `apply φ, …` | the named lemmas at the stated arguments, then `decide` |
| `lean => tac` | anything |

No closer tries another closer; a failing closer reports the goal it did not
close, in specification form. Only `solve` can spend the automatic budget.

## What a proof may mention

A step's expressions are the module's specification vocabulary: parameters
and locals by their Move names, `old(x)` for entry values, `result`, the
value written back to a local by a `&mut` borrow as `x'` (after the call
that borrows it; at an `ensures`, the final value), callee results as `g_result`, the
block's `let` bindings and quantifier binders, variant payload fields under
their Move names after a `split`, the module's specification functions and
lemmas, the built-in lemmas, and the map role functions (`spec_get`,
`spec_len`, …; see [Built-in lemmas](#built-in-lemmas)). No carrier
spelling, no normal form, no Lean lemma name appears unless the proof opts
into `lean =>`.

Preparation is the system's business. A proof states facts and names case
splits; the solver decides how contexts are prepared and may change it
without touching a proof.

### Built-in lemmas

The lemmas the language provides are named `lemma_…`; a map's lemmas take
their names from the module's own specification functions. A lemma's
premises — the map's validity, a position's bounds, a key's presence — are
owed by `decide` from the facts in scope; the tables below state the
conclusion each lemma gives.

**Map lemmas.** A storage-backed container is verified as an abstract map:
its spec block marks it `pragma intrinsic = map` and binds the map theory's
*roles* to functions of its module —

```text
pragma intrinsic = map,
    map_spec_get = spec_get,       // the `get` role
    map_spec_set = spec_set,       // the `set` role
    map_spec_del = spec_remove,    // the `del` role
    map_spec_len = spec_len,       // ...
    map_spec_has_key = spec_contains_key,
    map_spec_key_at = spec_key_at,
    map_spec_rank = spec_rank,
    map_spec_new = spec_new;
```

Each lemma reads certain roles of the map: the lemma of a removal's length
reads `len` and `del`, the lemma of a position's key reads `key_at`. An owner
gets a lemma exactly when it binds every role that lemma reads, and the
lemma is named `lemma_` followed by the bound functions' names without
`spec_`: an owner binding `spec_len` and `spec_remove` gets
`lemma_len_remove`; one binding `spec_get` and `spec_set` gets
`lemma_get_set`; one binding `spec_contains_key` and `spec_key_at` gets
`lemma_contains_key_key_at`. The `m`, `k`, `v`, `p`, `i`, `j` below are the
lemma's parameters, at the map's key and value types; a lemma whose
statement mentions `spec_rank` reads that role too.

| Lemma | Conclusion |
|---|---|
| `lemma_get_set(m, k, v, p)` | `spec_get(spec_set(m, k, v), p) == (if p == k then v else spec_get(m, p))` |
| `lemma_contains_key_set(m, k, v, p)` | `spec_contains_key(spec_set(m, k, v), p) == (p == k \|\| spec_contains_key(m, p))` |
| `lemma_len_set(m, k, v)` | `spec_len(spec_set(m, k, v)) == (if spec_contains_key(m, k) then spec_len(m) else spec_len(m) + 1)` |
| `lemma_key_at_set_present(m, k, v, i)` | `k` present: `spec_key_at(spec_set(m, k, v), i) == spec_key_at(m, i)` |
| `lemma_get_remove(m, k, p)` | `p != k ==> spec_get(spec_remove(m, k), p) == spec_get(m, p)` |
| `lemma_contains_key_remove(m, k, p)` | `spec_contains_key(spec_remove(m, k), p) == (p != k && spec_contains_key(m, p))` |
| `lemma_len_remove(m, k)` | `spec_len(spec_remove(m, k)) == (if spec_contains_key(m, k) then spec_len(m) - 1 else spec_len(m))` |
| `lemma_key_at_remove(m, k, i)` | ordered maps: `spec_key_at(spec_remove(m, k), i) == spec_key_at(m, if i < spec_rank(m, k) then i else i + 1)` |
| `lemma_len_new()` | `spec_len(spec_new()) == 0` |
| `lemma_contains_key_new(k)` | `spec_contains_key(spec_new(), k) == false` |
| `lemma_contains_key_key_at(m, i)` | `0 <= i && i < spec_len(m) ==> spec_contains_key(m, spec_key_at(m, i))` |
| `lemma_rank_key_at(m, i)` | `spec_rank(m, spec_key_at(m, i)) == i` |
| `lemma_key_at_rank(m, k)` | `k` present: `spec_key_at(m, spec_rank(m, k)) == k` |
| `lemma_key_at_eq_iff(m, i, j)` | `spec_key_at(m, i) == spec_key_at(m, j) <==> i == j` |
| `lemma_rank_lt_len(m, k)` | `k` present: `spec_rank(m, k) < spec_len(m)` |
| `lemma_rank_nonneg(m, k)` | `0 <= spec_rank(m, k)` |
| `lemma_rank_lt_rank(m, a, b)` | integer keys, `a < b`: `spec_rank(m, a) < spec_rank(m, b)` |
| `lemma_key_at_lt(m, i, j)` | integer keys, `i < j`: `spec_key_at(m, i) < spec_key_at(m, j)` |
| `lemma_key_at_set_before(m, k, v, i)` | ordered, `k` missing, `i` before `k`'s new position: `spec_key_at(spec_set(m, k, v), i) == spec_key_at(m, i)` |
| `lemma_key_at_set_after(m, k, v, i)` | ordered, `k` missing, `i` after `k`'s new position: `spec_key_at(spec_set(m, k, v), i) == spec_key_at(m, i - 1)` |

How the lemmas compose. Writing a value through an iterator at a position of
`OrderedMap` — the borrow resolves the position to a key, so the write lands
on that key and moves nothing:

```move
fun test_verify_iter_borrow_mut_at_position(m: &mut OrderedMap<u64, u64>, i: u64) {
    let it = IteratorPtr::Position { index: i };
    *it.iter_borrow_mut(m) = 7;
}

spec test_verify_iter_borrow_mut_at_position {
    requires i < spec_len(m);
    aborts_if false;
    ensures spec_get(m, spec_key_at(old(m), i)) == 7;
    ensures forall j in 0..spec_len(m): spec_key_at(m, j) == spec_key_at(old(m), j);
    ensures forall j in 0..spec_len(m) where j != i:
        spec_get(m, spec_key_at(m, j)) == old(spec_get(m, spec_key_at(m, j)));
}
```

The proof, from `ordered_map.proof.lean`. The second `ensures` — keys
unchanged — needs the key at `i` present and a write at a present key
preserving every position; the third — values unchanged away from `i` —
additionally needs the value lemma of the write and that two positions hold
the same key only when equal; the first — the written value — is the value
lemma alone:

```text
verify test_verify_iter_borrow_mut_at_position by
  on ensures 2 {
    let k := spec_key_at(m, i)
    apply lemma_contains_key_key_at(m, i)
    apply lemma_key_at_set_present(m, k, 7, j)
  }
  on ensures 3 {
    let k := spec_key_at(m, i)
    let kj := spec_key_at(m, j)
    apply lemma_contains_key_key_at(m, i)
    apply lemma_len_set(m, k, 7)
    apply lemma_key_at_set_present(m, k, 7, j)
    apply lemma_get_set(m, k, 7, kj)
    apply lemma_key_at_eq_iff(m, j, i)
  }
  on ensures 1 {
    let k := spec_key_at(m, i)
    apply lemma_get_set(m, k, 7, k)
  }
```

Each `apply` states one lemma at explicit arguments; the premises — the
position bounds, the key's presence — close by `decide` from the `requires`
and the loop of facts before it, and each `ensures` closes by `decide` over
the conclusions. The `aborts_if false` clause is proved by no block at all:
unnamed, it gets the cheap deciders alone, which close it — the iterator is
constructed a `Position`, so the callee's end-check cannot fire, and
`requires i < spec_len(m)` bounds the access behind the role. A map the
context determines completely is decided outright by `eval`, no lemmas named.

**Arithmetic lemmas.** A fixed table over the specification's `*`, `/`, `%`,
`<<`, `>>`, with Move's semantics (truncation toward zero; where truncation
and Euclidean division differ, the lemma requires nonnegative operands):

| Lemma | Statement |
|---|---|
| `lemma_mul_nonneg(a, b)` | `0 <= a && 0 <= b ==> 0 <= a * b` |
| `lemma_mul_mono(a, b, c, d)` | `0 <= a && a <= b && 0 <= c && c <= d ==> a * c <= b * d` |
| `lemma_mul_strict_mono(a, b, c)` | `a < b && 0 < c ==> a * c < b * c` |
| `lemma_div_nonneg(x, c)` | `0 <= x && 0 < c ==> 0 <= x / c` |
| `lemma_div_le(x, q, c)` | `0 <= x && 0 < c && x <= q * c ==> x / c <= q` |
| `lemma_div_lt(x, q, c)` | `0 <= x && 0 < c && x < q * c ==> x / c < q` |
| `lemma_le_div(x, q, c)` | `0 <= x && 0 < c && q * c <= x ==> q <= x / c` |
| `lemma_div_mono(x, y, c)` | `0 <= x && x <= y && 0 < c ==> x / c <= y / c` |
| `lemma_div_mul_le(x, c)` | `0 <= x && 0 < c ==> x / c * c <= x` |
| `lemma_div_add_mul(x, b, c)` | `0 <= x && 0 <= b && 0 < c ==> (x + b * c) / c == x / c + b` |
| `lemma_mod_small(x, c)` | `0 <= x && x < c ==> x % c == x` |
| `lemma_shl_mul(x, k)` | `x << k == x * 2^k` |
| `lemma_shr_div(x, k)` | `0 <= x ==> x >> k == x / 2^k` |

`arith` treats a product or quotient as an atom, so the pattern of a proof
over them is one `apply` per bound, then linear reasoning.

**Vector lemmas.** Each of Move's vector operations — `update`,
`push_back`, `pop_back`, `swap`, `remove`, `swap_remove`, the slice
`v[a..b]`, `concat` — has a length lemma `lemma_vector_len_<op>` stating
the result's length and an element lemma `lemma_vector_index_<op>(v, i)`
stating `v'[i]` case-wise by the position's relation to the operation's
index. The `vector_` keeps them apart from the map lemmas of an owner whose
role functions are named alike. For `remove`:

```text
lemma_vector_len_remove(v, i)     : len(remove(v, i)) == len(v) - 1
lemma_vector_index_remove(v, i, k): k < i ==> v'[k] == v[k]
                                    i <= k ==> v'[k] == v[k + 1]
```

Extensionality closes equalities of whole vectors:

```text
lemma_vector_ext(v, w): len(v) == len(w)
                        && (∀ (k : Int), 0 <= k && k < len(v) ==> v[k] == w[k])
                        ==> v == w
```

A recursive specification function's unfolding is a lemma of it
(`apply spec_sum_upto.unfold(m, n)`).

### Lemma items

A `lemma` item is a `spec lemma` stated beside the source instead of in it,
with the same `requires`/`ensures` clauses and a block of steps:

```text
lemma effective_bound(a : Int, scale : Int)
  requires 0 <= a && 0 <= scale && scale <= 10000
  ensures 0 <= a * scale / 10000 && a * scale / 10000 <= a
by {
  apply lemma_mul_nonneg(a, scale)
  apply lemma_mul_mono(a, a, scale, 10000)
  apply lemma_div_nonneg(a * scale, 10000)
  apply lemma_div_le(a * scale, a, 10000)
}
```

Each `apply` owes its premise by `decide` from the facts before it, and the
`ensures` closes by `decide` over the four conclusions. A lemma of a
recursion group names its `decreases` measure and is proved by induction
through `apply` of itself, as a `spec lemma` in the source is:

```lean
spec lemma sum_bound(n : Int) decreases n where
  requires 0 <= n
  ensures sum(n) >= n
  proof
    split n > 0
    apply n > 0 ==> sum_bound(n - 1)
```

`apply` reaches a file's lemmas as it reaches the module's.

## The escape

`lean =>` runs a Lean tactic block where it stands — as a closer (`assert e
by lean => …`), as a step, or as a whole item (`verify f by lean => …`). It
is for arguments that are genuinely about Lean terms; inside it the goal has
the solver's prepared form. A proof that needs it signals a gap in the
language or the lemma set, to be fixed there. The `proofs` fixture's
`square_of_sum` is such an item today, a rewriting chain where the `ring`
proof above is the intended form:

```lean
verify square_of_sum by lean =>
  verex_obligations
  verex_on
  | ensures_1 =>
    verex_prepare
    rw [Int.add_mul, Int.mul_add, Int.mul_add, Int.mul_comm b.val a.val, Int.mul_assoc]
    omega
  | _ => …
```

## Writing proofs from a failure

The workflow needs only the Move file and the failure message: the message
names each open obligation by its key, states it and the facts available in
specification form, and prints the item as a template. Read the Move code at
the named clause or site, state the facts that connect the available ones to
the goal — by `apply` of the built-in lemmas and the module's own — and let
`decide` close. Each step fails individually with the fact it owed, so repair
is local.
