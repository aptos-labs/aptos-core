# pass
cargo run --release -p prover-lab -- bench -f test.move

# pass
cargo run --release -p prover-lab -- bench -f -c prover.toml test.move

# pass
 cargo run --release -p prover-lab -- bench -f -c prover.toml -d ../../../../move-stdlib/sources test_stdlib.move

# pass
cargo run --release -p prover-lab -- bench -f -c prover_stdlib.toml test.move

# expect to fail
cargo run --release -p prover-lab -- bench -f -c prover.toml test_stdlib.move
