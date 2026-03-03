# README

## Move unit tests

To run, use the following command in this directory:
```
TEST_FILTER=conf cargo test -- experimental --skip prover
```

## Gas benchmarks

Relative to the root of the `aptos-core` repository, run:
```
cargo run -p aptos-framework -- update-cached-packages --with-test-mode

cd aptos-move/e2e-move-tests/src/
cargo test --features move-harness-with-test-only -- bench_gas --nocapture
```

## Some limitations of Move that had to be worked around

 - Variables cannot start with a capital letter: e.g., _G_ must be turned into _\_G_
 - Cannot have `Statement` and `CompressedStatement` structs declared in one Move file that both have a `get_num_scalars(self)` function
 - Cannot add more levels of scope (e.g., `aptos_experimental::sigma_protocols::statement`)
 - Function values still force me to use inlining
 - Cannot have an `Option<(RistrettoPoint, CompressedRistretto)>` type.
 - Friend module `F` of module `B` cannot directly access fields of structs declared in `B`
    + Coupled with the fact that two different structs cannot have the same named function if declared in the same module, this makes modular design in Move very difficult
        * I can either put everything in the same module and deal with the naming conflicts (e.g., `new_proof` and `new_statement`) ==> not modular
        * Or, I can put things in different modules but now have to declare artificial setters and getters ==> blows up code size
 - **Resolved:** No `last()` or `back()` method to fetch the last element in a vector. (I added one.)
 - `public fun add_assign_pending(self: &mut CompressedBalance<Pending>, rhs: &Balance<Pending>)` does not compile when using `self`
