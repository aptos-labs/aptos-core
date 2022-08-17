.PHONY: build
build:
	cargo build --target-dir ./target --target wasm32-unknown-unknown --release

.PHONY: stream
stream:
	substreams run -p -e localhost:18015 substreams.yaml block_to_block_output --start-block 0 --stop-block +100

.PHONY: codegen
codegen:
	substreams protogen ./substreams.yaml --exclude-paths="sf/substreams,google"