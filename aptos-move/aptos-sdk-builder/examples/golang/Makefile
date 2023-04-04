#Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

clean:
	- rm -rf go.mod go.sum aptosstdlib aptostypes

test:
	cargo run -p aptos-framework release
	cargo run -p aptos-sdk-builder -- \
		--language go --module-name aptosstdlib \
		--package-name testing \
		--with-aptos-types "../../../../testsuite/generate-format/tests/staged/aptos.yaml" \
		--target-source-dir . \
		"../../../framework/aptos-framework/build/AptosFramework"
	- go mod init testing
	go mod edit -replace testing=`pwd`
	go get github.com/aptos-labs/serde-reflection/serde-generate/runtime/golang/bcs
	go get github.com/aptos-labs/serde-reflection/serde-generate/runtime/golang/serde
	go run stdlib_demo.go

.PHONY: clean test
