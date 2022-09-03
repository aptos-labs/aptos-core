.PHONY: build
build:
	cargo build --target-dir ./target --target wasm32-unknown-unknown --release

.PHONY: stream_block
stream_block:
	substreams run -p -e localhost:9000 substreams.yaml block_to_block_output --start-block 4466 --stop-block +0

.PHONY: stream_token
stream_token:
	substreams run -p -e localhost:9000 substreams.yaml block_output_to_token --start-block 64205 --stop-block +0