# Benchmarking current vs alternative version of Boogie

To examine the performance regression, this compares the current version of Boogie with an alternative, newer one.

- The `current_boogie` benchmark uses Boogie 2.15.8.
- The `new_boogie:` benchmark uses Boogie 2.16.9.
  This benchmark ran on top of the recent update on Prover's boogie backend (commit: 52ac38a7b6194567c4c82ae75f38a46ca03b5304).
- Both benchmarks use Z3 4.11.2.
- We ran both benchmarks three times each. `current_boogie_x` (`new_boogie_x`) indicates the result of the `x`th run.

## Module Verification Time

![Module-By-Module](mod_by_mod.svg)

## Function Verification Time

![Function-By-Function](fun_by_fun.svg)
