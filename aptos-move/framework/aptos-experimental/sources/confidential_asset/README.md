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
 - [Resolved] `cargo test` in `aptos-experimental/` fails to compile but `aptos move compile works`: this is because it also compiles the tests/ which `aptos` does not.
 - `map()` does not work with `NewElement` set to `()`; e.g., the following code does not compile
 - friend `F` of module `B` cannot access fields directly of structs declared in `B`
    + Coupled with the fact that two different structs cannot have the same named function if declared in the same module, this makes modular design in Move a nightmare
        * you either put everything in the same module and deal with the naming conflicts (e.g., `new_proof` and `new_statemetn`) ==> not modular
        * or you put things in different modules and now you gotta declare setters and accessors like crazy ==> blows up code size

The code:
```
	/// Deserializes a vector of point bytes to a vector of RistrettoPoints and a vector of their compressed counterparts.
    public fun deserialize_points(points_bytes: vector<vector<u8>>): (vector<RistrettoPoint>, vector<CompressedRistretto>) {
        let points = vector[];
        let compressed_points = vector[];
        points_bytes.map(|point_bytes| {
            let (point, compressed_point) = ristretto255::new_point_and_compressed_from_bytes(point_bytes);

            points.push_back(point);
            compressed_points.push_back(compressed_point);
        });

        (points, compressed_points)
    }
```

The error:
```
error: tuple type `()` is not allowed as a type argument
    ┌─ /Users/alinush/repos/aptos-core/aptos-move/framework/aptos-experimental/sources/confidential_asset/sigma_protocols/sigma_protocol_utils.move:68:9
    │
 68 │ ╭         points_bytes.map(|point_bytes| {
 69 │ │             let (point, compressed_point) = ristretto255::new_point_and_compressed_from_bytes(point_bytes);
 70 │ │
 71 │ │             points.push_back(point);
 72 │ │             compressed_points.push_back(compressed_point);
 73 │ │         });
    │ ╰──────────^
    │
    ┌─ /Users/alinush/repos/aptos-core/aptos-move/framework/aptos-experimental/../aptos-framework/../aptos-stdlib/../move-stdlib/sources/vector.move:545:36
    │
545 │     public inline fun map<Element, NewElement>(
    │                                    ---------- declaration of type parameter `NewElement`
    │
    = required by instantiating type parameter `NewElement` of function `map`

{
  "Error": "Move compilation failed: exiting with context checking errors"
}
```
