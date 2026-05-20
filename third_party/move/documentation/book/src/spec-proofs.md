# Proofs and Inference

_Since language version 2.4_

This chapter describes the proof toolkit available alongside automated
verification: explicit proof blocks and reusable lemmas that guide the SMT
solver through hard verification tasks, and a weakest-precondition based
inference engine that can derive specifications from code.

## Proofs and Lemmas

### Overview

Some verification tasks are too hard for an SMT solver to discharge automatically. The solver may time out on non-linear arithmetic, need a case split the heuristics miss, or require an intermediate fact that bridges a gap in reasoning. MSL addresses this with two complementary features:

- **Proof blocks** (`proof { ... }`) attached to a function specification, containing structured hints — assertions, assumptions, case splits, and calculational chains — that guide the solver step by step.
- **Lemma declarations** (`lemma name(...) { ... }`) that state reusable theorems with their own specifications and optional proof bodies. Lemmas are applied at proof sites with `apply` or `forall...apply`.

Together, these features let developers express proof strategies that would otherwise require restructuring the code or adding ghost state.

### Proof Blocks

A proof block is attached to a `spec` block with the `proof` keyword:

```move
fun double(x: u64): u64 {
    x + x
}
spec double {
    aborts_if 2 * x > MAX_U64;
    ensures result == 2 * x;
} proof {
    assert x + x == 2 * x;
}
```

The proof block contains *proof statements* that are emitted as assumptions and assertions during verification. Statements in a proof block execute in two contexts:

- **Entry context** (default): evaluated at the function's entry point, before the body runs. The variable `result` is not available.
- **Post context** (via `post`): evaluated at the function's return point, after the body runs. Both `result` and `old()` are available.

Proof blocks are local to the specification they are attached to. They do not change the executable Move code; they only influence verification by adding auxiliary verification conditions and assumptions.

### Proof Statement Summary

The following table summarizes the proof statements currently supported:

| Statement | Context | Effect |
|-----------|---------|--------|
| `assert e;` | entry, post | Prove `e` as a separate verification condition |
| `assume [trusted] e;` | entry, post | Inject `e` as a trusted assumption; emits a warning |
| `let x = e;` | entry, post | Bind a local proof name for later statements in the same scope |
| `if (c) { ... } else { ... }` | entry, post | Split the proof into cases under the corresponding path conditions |
| `post stmt` / `post { ... }` | entry only | Move the enclosed statement(s) to return-point checking |
| `calc(e1 == e2 <= e3 ...);` | entry, post | Emit one verification condition per chain step |
| `split e;` | entry, post | Create separate verification variants for boolean or enum cases |
| `apply lemma(args);` | entry, post | Assert lemma preconditions and assume lemma postconditions |
| `forall ... apply lemma(args);` | entry, post | Introduce a quantified lemma instantiation, optionally with triggers |

As a rule of thumb:

- Use `assert` when the solver is missing an intermediate fact.
- Use `calc` when a proof is primarily algebraic rewriting.
- Use `if` or `split` when the prover needs explicit case analysis.
- Use `apply` when you want to reuse a previously established theorem.

### `assert`

An `assert` in a proof block emits a verification condition — the solver must prove it holds. Assertions serve as intermediate lemmas that break a hard proof into smaller steps:

```move
fun weighted_avg_x2(x: u64, y: u64): u64 {
    (3 * x + y) / 4 * 2
}
spec weighted_avg_x2 {
    requires 3 * x + y <= MAX_U64;
    ensures result == (3 * x + y) / 4 * 2;
    ensures result <= 3 * x + y;
} proof {
    let wx = 3 * x;
    let sum = wx + y;
    let half = sum / 4;
    assert half <= sum;
    assert half * 2 <= sum;
}
```

Each `assert` establishes a fact that helps the solver reach the postconditions step by step.

### `assume [trusted]`

An `assume [trusted]` introduces a fact without proof. This is an escape hatch for properties the SMT solver cannot derive on its own. Trusted assumptions are unsound if wrong — use them sparingly and only for well-understood mathematical facts:

```move
fun div3_le(x: u64): u64 { x / 3 }
spec div3_le {
    ensures result <= x;
} proof {
    assume [trusted] x / 3 <= x;
}
```

The `[trusted]` annotation is required. An unannotated `assume` is rejected so that trusted assumptions are always explicit in the source.

### `let` Bindings

A `let` in a proof block names an intermediate value for use in subsequent statements. Let bindings are scoped to their enclosing block:

```move
fun square_plus_one(x: u64): u64 {
    (x + 1) * (x + 1)
}
spec square_plus_one {
    requires x + 1 <= 4294967295;
    ensures result == (x + 1) * (x + 1);
} proof {
    let y = x + 1;
    let r = y * y;
    assert r == (x + 1) * (x + 1);
    post assert r == result;
}
```

Let bindings make complex proofs readable by giving names to subexpressions.

- A binding introduced at entry can be used by later entry statements and by later `post` statements.
- A binding introduced inside a nested block is scoped to that block.
- A binding introduced inside `post { ... }` is available only inside that post block.

### `if`/`else` Case Splits

An `if`/`else` in a proof block splits the verification into branches. Each branch adds its condition as an assumption, letting the solver reason about cases independently:

```move
fun max(a: u64, b: u64): u64 {
    if (a >= b) { a } else { b }
}
spec max {
    ensures result >= a;
    ensures result >= b;
    ensures result == a || result == b;
} proof {
    if (a >= b) {
        post assert result == a;
        assert a >= a;
        assert a >= b;
    } else {
        post assert result == b;
        assert b > a;
        assert b >= b;
    }
}
```

### `post` Statements

The `post` prefix moves a statement to the return-point context, where `result` and `old()` are available. Without `post`, statements execute at the entry point where `result` is not yet defined:

```move
fun double(x: u64): u64 {
    x + x
}
spec double {
    requires x + x <= MAX_U64;
    ensures result == 2 * x;
} proof {
    // Entry-point assertion (no result available)
    assert x + x == 2 * x;
    // Return-point assertion (result available)
    post assert result == x + x;
}
```

A `post` block groups multiple post-context statements together:

```move
fun shift_add(x: u64, y: u64): u64 {
    x * 2 + y
}
spec shift_add {
    requires x * 2 + y <= MAX_U64;
    ensures result == x * 2 + y;
} proof {
    let doubled = x * 2;
    assert doubled + y <= MAX_U64;
    post {
        let expected = doubled + y;
        assert result == expected;
    }
}
```

Let bindings defined at entry are available inside `post` blocks. Let bindings inside a `post` block are scoped to that block.

`post` is intended for function proofs. Because lemmas have no return value, `post` statements are not meaningful inside lemma proofs.

### `calc` Chains

A `calc(...)` statement expresses a step-by-step chain of equalities or inequalities. Each step is a separate verification condition, and the chain's conclusion follows by transitivity:

```move
fun add_three(x: u64): u64 {
    x + 1 + 1 + 1
}
spec add_three {
    requires x + 3 <= MAX_U64;
    ensures result == x + 3;
} proof {
    calc(
        x + 1 + 1 + 1
        == x + 2 + 1
        == x + 3
    );
}
```

Calc chains support mixed operators (`==`, `<=`, `>=`). The overall relation is the weakest one in the chain:

```move
fun double_plus_one(x: u64): u64 {
    2 * x + 1
}
spec double_plus_one {
    requires 2 * x + 1 <= MAX_U64;
    ensures result >= x;
} proof {
    calc(
        2 * x + 1
        >= 2 * x
        >= x
    );
}
```

If any step in the chain is wrong, the prover reports an error at that specific step.

### `split`

The `split` statement generates separate verification variants for each possible value of an expression. For a boolean expression, this creates two variants (true/false). For an enum, it creates one variant per constructor:

```move
fun abs_diff(a: u64, b: u64): u64 {
    if (a >= b) { a - b } else { b - a }
}
spec abs_diff {
    ensures result == if (a >= b) { a - b } else { b - a };
} proof {
    split a >= b;
}
```

Splitting on an enum:

```move
enum Color has drop {
    Red,
    Green,
    Blue,
}

fun color_code(c: Color): u64 {
    match (c) {
        Color::Red => 1,
        Color::Green => 2,
        Color::Blue => 3,
    }
}
spec color_code {
    ensures result >= 1;
    ensures result <= 3;
} proof {
    split c;
}
```

Each variant assumes the corresponding case and must independently satisfy all postconditions. If the postcondition is too strong for any variant, the prover reports an error for that variant.

Practical notes:

- The split expression must have type `bool` or an enum type.
- Multiple `split` statements multiply the number of verification variants, so they should be used sparingly.
- `split` is most useful when the function body already branches on the same condition or enum and the solver is failing to connect the cases.

### Lemma Declarations

A lemma is a reusable theorem declared with `spec lemma`. It has a name, typed parameters, specification conditions (`requires`/`ensures`), and an optional proof body. Lemmas are specification-only declarations: they are not executable Move functions, and their result type is implicitly `()`. Without a proof body, the lemma is discharged as a verification condition — the prover must prove it holds for all inputs satisfying the preconditions. With a proof body, the proof block provides hints to guide the solver.

Here is a lemma proving that a recursive sum function is monotone. The proof is inductive — it applies itself on a smaller argument:

```move
spec fun sum(n: num): num {
    if (n == 0) { 0 } else { n + sum(n - 1) }
}

spec lemma monotonicity(x: num, y: num) {
    requires 0 <= x;
    requires x <= y;
    ensures sum(x) <= sum(y);
} proof {
    if (x < y) {
        assert sum(y - 1) <= sum(y);
        apply monotonicity(x, y - 1);
    }
}
```

The base case (`x == y`) is trivial. The inductive step assumes `x < y`, asserts a one-step fact about `sum`, and recurses on `(x, y - 1)`. The prover verifies both cases.

### `apply` and `forall...apply`

The `apply` statement instantiates a lemma at a proof site. Operationally, it does two things:

1. It emits proof obligations for the lemma's `requires` clauses at the application site.
2. It makes the lemma's `ensures` clauses available to the remainder of the current proof.

Multiple `apply` statements can be chained — each one's conclusions are available to subsequent steps. If a lemma's preconditions are not satisfied, the prover reports an error at the `apply` site.

The `forall...apply` variant instantiates a lemma universally for all values of the quantified variables. This is essential when the lemma needs to be available across all iterations of a loop or for a recursive function. Using the `monotonicity` lemma from above:

```move
fun sum_up_to(n: u64): u64 {
    if (n == 0) { 0 }
    else { n + sum_up_to(n - 1) }
}
spec sum_up_to {
    aborts_if sum(n) > MAX_U64;
    ensures result == sum(n);
} proof {
    forall x: num, y: num {sum(x), sum(y)} apply monotonicity(x, y);
}
```

The `{sum(x), sum(y)}` clause provides *triggers* — patterns that tell the SMT solver when to instantiate the quantified lemma. The solver will instantiate `monotonicity(x, y)` whenever it encounters terms matching `sum(x)` and `sum(y)`. Without triggers, the solver may fail to instantiate the quantified lemma at the right points.

In practice:

- Prefer a plain `apply` when you need a theorem only for the current concrete arguments.
- Use `forall...apply` when the proof needs a quantified fact that must be available at many later uses.
- Add triggers when the quantified lemma mentions recursive spec functions or other terms the solver is unlikely to instantiate on its own.

### Proofs with `&mut` Parameters

Proof blocks work with functions that take mutable reference parameters. In the entry context, `&mut` parameters refer to their original (pre-mutation) values. In the post context, `old()` accesses the pre-mutation value and the bare name accesses the post-mutation value:

```move
struct Counter has drop {
    value: u64,
}

fun increment(c: &mut Counter) {
    c.value = c.value + 1;
}
spec increment {
    requires c.value < MAX_U64;
    ensures c.value == old(c.value) + 1;
} proof {
    // Entry context: c.value is the original value
    assert c.value < MAX_U64;
    assert c.value + 1 <= MAX_U64;
}
```

Post-context statements can use `old()` to relate pre and post states, and `result` for return values:

```move
fun add_and_return(c: &mut Counter, n: u64): u64 {
    c.value = c.value + n;
    c.value
}
spec add_and_return {
    requires c.value + n <= MAX_U64;
    ensures c.value == old(c.value) + n;
    ensures result == c.value;
} proof {
    assert c.value + n <= MAX_U64;
    post assert c.value == old(c.value) + n;
    post assert result == c.value;
}
```

Lemmas can be applied at post points with `old()` arguments to relate pre and post state:

```move
spec lemma strict_increase(a: u64, b: u64) {
    requires b == a + 1;
    ensures a < b;
}

fun bump(c: &mut Counter) {
    c.value = c.value + 1;
}
spec bump {
    requires c.value < MAX_U64;
    ensures c.value == old(c.value) + 1;
    ensures old(c.value) < c.value;
} proof {
    post apply strict_increase(old(c.value), c.value);
}
```

### Lemma Shortcut Syntax

Lemmas can also be declared inside a `spec module { ... }` block using bare `lemma` (without the `spec` prefix). This is equivalent to top-level `spec lemma` and is useful when grouping multiple lemmas together:

```move
spec module {
    lemma add_zero_left(x: u64) {
        ensures 0 + x == x;
    }

    lemma mul_comm(a: u64, b: u64) {
        ensures a * b == b * a;
    }
}
```

Note that if a function is literally named `lemma`, `spec lemma { ... }` (with `{` immediately after `lemma`) is parsed as the function's spec block, not a lemma declaration, since the parser looks for an identifier after `lemma` to distinguish the two forms.

## Specification Inference

With these constructs, the Move Prover now includes a *specification inference engine* that can automatically derive specifications for functions. This is a key ingredient for scaling formal verification: rather than requiring developers to manually annotate every function, the prover can infer many specifications automatically.

### Weakest-Precondition Approach

The inference engine uses a _weakest-precondition_ (WP) backward analysis over the function's bytecode. Starting from the function's exit point, it works backward through each instruction, accumulating the conditions that must hold for the function to satisfy its specification. For each state-changing operation (global writes, function calls, aborts), the analysis emits an appropriate specification condition:

- **Direct mutations** (e.g., `Counter[addr].value = v`) produce `ensures` conditions that relate the final state to the initial state using `update_field`.
- **Opaque function calls** produce behavioral predicate conditions (`ensures_of`, `aborts_of`) that delegate to the callee's specification.
- **Abort points** produce `aborts_if` conditions.
- **Frame conditions** (`modifies`) are inferred from the set of global resources written by the function.

When multiple state-changing operations occur in sequence, the inference engine introduces [state labels](./spec-compositional.md#state-labels) to distinguish intermediate states, producing a chain of constraints (e.g., `..S |~ ensures_of<f>(a)` followed by `S.. |~ ensures_of<g>(b)`).

### Loops and Invariants

The WP analysis requires that each loop in the function body has an explicit _loop invariant_. Without invariants, the analysis cannot reason across loop iterations and will produce vacuous (trivially true) specifications that are unsound for verification.

If a function contains loops without invariants, the inference engine will still produce output, but the inferred specifications should be treated as incomplete. For correct results, loop invariants must be provided by the developer (or AI -- see below) before inference can derive meaningful specifications.

### Integration with AI-Based Inference

The WP-based inference engine is designed to complement AI-based specification inference, specifically for loop invariants, as provided by the **MoveFlow** tool. The two approaches have complementary strengths:

- **WP inference** is precise and sound for straight-line code with opaque calls and mutations, but requires human-provided loop invariants and cannot guess high-level intent.
- **AI-based inference** can suggest loop invariants, high-level properties, and specifications for functions where WP analysis alone is insufficient.

A typical workflow combines both: AI-based tools propose candidate specifications (including loop invariants), and the WP engine fills in the precise arithmetic and frame conditions. The result is then verified by the Move Prover to ensure soundness.
