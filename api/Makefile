# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

test: clean lint test-code-gen clean

lint:
	npx @redocly/openapi-cli lint doc/openapi.yaml --skip-rule no-empty-servers

test-code-gen:
	npx @openapitools/openapi-generator-cli version-manager set 5.2.1
	npx @openapitools/openapi-generator-cli generate -g rust -i doc/openapi.yaml -o /tmp/diem_api_client --package-name diem_api_client
	cd /tmp/diem_api_client && cargo build

clean:
	rm -rf /tmp/diem_api_client
	rm -rf openapitools.json

.PHONY: test lint test-code-gen clean
