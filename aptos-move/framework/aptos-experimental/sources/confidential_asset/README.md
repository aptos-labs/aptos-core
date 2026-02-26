# README

## Move unit tests

To run, use the following command in this directory:
```
TEST_FILTER=conf cargo test -- experimental --skip prover
```

## Gas benchmarks

Relative to the root of the `aptos-core` repository, run:
```
cd aptos-move/e2e-move-tests/src/
cargo test -- bench_gas
```

## Limitations of Move

 - Variables cannot start with a capital letter: e.g., _G_ must be turned into _\_G_
 - Cannot have `Statement` and `CompressedStatement` structs declared in one Move file that both have a `get_num_scalars(self)` function
 - Cannot add more levels of scope (e.g., `aptos_experimental::sigma_protocols::statement`)
 - Function values still force me to use inlining
 - Cannot have an `Option<(RistrettoPoint, CompressedRistretto)>` type.
 - friend `F` of module `B` cannot access fields directly of structs declared in `B`
    + Coupled with the fact that two different structs cannot have the same named function if declared in the same module, this makes modular design in Move a nightmare
        * you either put everything in the same module and deal with the naming conflicts (e.g., `new_proof` and `new_statemetn`) ==> not modular
        * or you put things in different modules and now you gotta declare setters and accessors like crazy ==> blows up code size
 - [Resolved] `cargo test` in `aptos-experimental/` fails to compile but `aptos move compile works`: this is because it also compiles the tests/ which `aptos` does not.
