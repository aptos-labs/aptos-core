{# Proofs, lemmas, and proof syntax #}
{% if once(name="spec_lang_proofs") %}

## Proof guidance

Use proof structure when a correct contract times out or the solver cannot find
an intermediate fact:

- `assert e` exposes a useful sub-goal and must itself be proved.
- `apply lemma(args)` instantiates a proved lemma.
- `forall x: T {trigger(x)} apply lemma(x)` applies a lemma universally with
  an explicit trigger.
- `calc` records an equality or inequality chain.
- conditionals and value splits separate materially different proof cases.

Prefer small assertions and lemmas tied to the failing obligation. A lemma is a
proved module-level proposition, not an axiom:

```move
spec module {
    fun sum(values: vector<u64>, n: num): num {
        if (n == 0) { 0 } else { sum(values, n - 1) + values[n - 1] }
    }

    lemma sum_step(values: vector<u64>, n: num) {
        requires 0 < n && n <= len(values);
        ensures sum(values, n) == sum(values, n - 1) + values[n - 1];
    }
}
```

Attach `proof { ... }` after a function spec or lemma.
{% if evaluation_mode %}
Do not add `assume`, an axiom, or an unproved native helper: such constructs
replace a proof obligation rather than solve it and are forbidden in evaluation
mode.
{% else %}
Do not add `assume`, an axiom, or an unproved native helper to make a condition
pass unless the user or an explicit project policy establishes that trusted
boundary; record it clearly because it replaces a proof obligation.
{% endif %}

Only uninterpreted-function applications are valid quantifier triggers. Prefer
quantifier-free formulations when they express the same contract.

### Instantiation weight

A recursive specification function is encoded as a defining axiom the solver
unrolls whenever one of its applications appears, and a `forall ... apply`
is a quantifier the solver instantiates at every match. Either can dominate a
timeout, which the analysis reports as a `definition of spec function` or a
`forall` entry. `[weight = N]` raises the cost the solver charges for each
instantiation, so it unrolls or instantiates only when nothing cheaper
remains; it changes no proof semantics.

```move
spec module {
    fun count(v: vector<u64>, x: num, k: num): num [weight = 20] {
        if (k <= 0) { 0 } else { count(v, x, k - 1) + (if (v[k - 1] == x) { 1 } else { 0 }) }
    }
}
spec f {
    ...
} proof {
    forall v: vector<u64>, i: num, j: num, x: num
        {count(update(update(v, i, v[j]), j, v[i]), x, len(v))} [weight = 20]
        apply count_swap(v, i, j, x);
}
```

Use it when the facts the proof needs come from lemmas applied one step at a
time and the definition should not be unrolled on its own. A weight of 20 is a
reasonable start. Without it the same contract can prove but leave every
refutation -- a wrong implementation against a correct contract -- exhausting
the budget instead of failing.

{% endif %}
