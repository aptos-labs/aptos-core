{# Proofs, lemmas, and proof syntax — split from spec_lang.md #}
{% if once(name="spec_lang_proofs") %}

## Proofs and Lemmas

### Example

```move
spec fun sum(n: u64): u64 {
    if (n == 0) { 0 } else { n + sum(n - 1) }
}

spec lemma monotonicity(x: num, y: num) {
    requires x <= y;
    ensures sum(x) <= sum(y);
} proof {
    if (x < y) {
        assert sum(y - 1) <= sum(y);
        apply monotonicity(x, y - 1);
    }
}


fun sum_up_to(n: u64): u64 { /* iterative impl */ }
spec sum_up_to {
    requires n <= 5;
    ensures result == sum(n);
} proof {
   forall x,y {sum(x), sum(y)} apply monotonicity(x, y);
}
```

### Proofs

A proof consists of a sequence of
proof statements together with if-then-else and let bindings.

Proof statements: `let name = Expr`, `if (Expr) Proof else Proof`,
`assert Expr`, `assume Expr`, `apply LemmaInstance`,
`forall QuantifierDecls [Patterns] apply LemmaInstance`,
`calc (Expr { RelOp Expr })`.

A proof block can be attached to any specification block as postfix to that block, for example:

```
spec sum_to_n {
  ensures result == sum(n);
} proof {
  forall x: u64, y: u64 apply Monotonicity(x, y);
}  
```

A proof is translated by mapping it to a sequence of assumes/asserts at the
verification entry points of a function.

- The split statement is translated by creating different verification variants for each value split 
  with according assumptions of the value at the split point and otherwise identical content.
- The apply statement is translated by injecting pre/post conditions of the (expected to be proven) lemma.
  This is very similar like calling an opaque function in Move code.

### Lemmas

A Lemma is a member of a specification block, similar like a spec function. Its
user syntax is:

```
spec fun sum(n: u64): u64 {
    if (n == 0) { 0 } else { n + sum(n - 1) }
}
spec lemma sum_monotonicity(x: num, y: num) {
    requires x <= y;
    ensures sum(x) <= sum(y);
} proof {
    if (x < y) {
        assert sum(y - 1) <= sum(y);
        apply sum_monotonicity(x, y - 1);
    }
}
```

Or inside a `spec module { }` block (the keyword `module`, not a module name):

```
spec module {
  fun sum ...
  lemma sum_monotonicity ...
}
```

**Important:** `spec name { }` always targets a *function* named `name`.
There is no `spec <module_name> { }` syntax. Module-level items (helper
functions, lemmas) go inside `spec module { }`. Lemmas are **not valid**
inside function spec blocks (`spec fun_name { }`).

The `spec lemma` shortcut is sugar for `spec module { lemma ... }`, analogous
to the `spec fun` shortcut for helper functions.

It has a parameter list like a spec function (but no return value) followed by a
specification block (with requires, ensures, and pragmas the only allowed conditions).
Attached to this is an (optional) proof.

Lemma names are in a separate namespace. They are scoped to modules,
similar like specification functions. They can only be referenced from
proof 'apply' statements.

{% endif %}
