# Inline coverage example

## Background

Fungible assets can theoretically be seized at any moment by the issuer (unless
said issuer destroys such capabilities at asset creation time), so in the case
of a seizure from a public pool it may be useful to socialize losses across
all depositors to prevent a bank run. For example:

1. Ace deposits 200 of a centralized stablecoin into pool X
1. Bee deposits 300 of a centralized stablecoin into pool X
1. The centralized stablecoin issuer seizes 250 of the 500 in pool X
1. Ace attempts to withdraw 200, but since the pool is only 50% collateralized,
   he only ends up receiving 100. (If the pool returned all 200, then Ace would
   have initiated a bank run and Bee would be in trouble).
1. Bee decides to wait, judging that the stablecoin issuer will top off the pool
   back to full collateralization, because if the issuer does not, then their
   reputation will be destroyed and market participants will favor the
   stablecoin of a competitor who does not arbitrarily seize from public pools.

## Algorithms

This example contains two Move modules, `full_coverage`, and `partial_coverage`.
Both modules contain the same simple function definition for a socialized
withdrawal from a pool, with one minor difference.

In each module, the socialized withdrawal function first performs several
well-formedness checks and simply returns the requested amount in the case of a
fully-collateralized pool. However, in the case that the pool is not fully
collateralized, the requested withdrawal amount is scaled by the
collateralization ratio of the pool itself, as in the background example above.

In `partial_coverage`, the scaling operation relies on `math64::mul_div`, while
in `full_coverage`, the scaling operation relies on `math64::mul_div_unchecked`.
Notably, only the latter module is able to achieve 100% coverage because the
`mul_div` function in the former asserts a nonzero divisor.

Note that if `mul_div` were a standard (non-inline) function, it wouldn't
prohibit a calling module from being tested to 100% coverage. However, since it
is an inline function, the code is effectively copy-pasted into the calling
function `partial_coverage::socialize_withdrawal_amount` during compile time,
including the assert statement to check for a nonzero divisor. Hence, since the
well-formedness checks in the beginning of the socialized withdrawal function
rule out the possibility of passing a divisor of zero to the `mul_div` function,
it is impossible to construct a corresponding `expected_failure` test and thus
impossible to achieve 100% coverage in the `partial_coverage` module.

Conversely, `full_coverage` relies on the `mul_div_unchecked` function, which
has no such assert statement, hence the module can be fully coverage tested.

## Coverage walkthrough

To coverage test both modules, run:

```sh
aptos move test --coverage --dev
```

This should return something like:

```sh
...
+-------------------------+
| Move Coverage Summary   |
+-------------------------+
Module ace::full_coverage
>>> % Module coverage: 100.00
Module ace::partial_coverage
>>> % Module coverage: 93.88
...
```

Noting that your tests have not achieved 100% coverage in one of your modules,
you would typically then run the following to view module source code colored
with coverage information:

```sh
aptos move coverage source --dev --module partial_coverage
```

However, as in this example, the `move coverage source` command does not always
produce intelligible results for inline functions that have not hit coverage, so
you'll actually have to compare against bytecode to identify the culprit:

```sh
aptos move coverage bytecode --dev --module partial_coverage
```

This reveals the coverage gap at:

```sh
        32: LdU64(4)
        33: Call error::invalid_argument(u64): u64
        34: Abort
```

Note that this branch corresponds to the assert statement from `mul_div`, which
is not present in the `full_coverage` module bytecode:

```sh
aptos move coverage bytecode --dev --module full_coverage
```
