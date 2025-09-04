#Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

clean:
	- rm -rf go.mod go.sum velorstdlib velortypes

test:
	cargo run -p velor-framework release
	cargo run -p velor-sdk-builder -- \
		--language go --module-name velorstdlib \
		--package-name testing \
		--with-velor-types "../../../../testsuite/generate-format/tests/staged/velor.yaml" \
		--target-source-dir . \
		"../../../framework/velor-framework/build/VelorFramework"
	- go mod init testing
	go mod edit -replace testing=`pwd`
	go get github.com/velor-chain/serde-reflection/serde-generate/runtime/golang/bcs
	go get github.com/velor-chain/serde-reflection/serde-generate/runtime/golang/serde
	go run stdlib_demo.go

.PHONY: clean test
