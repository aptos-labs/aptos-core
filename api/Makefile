# Copyright © Aptos Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0

test: clean lint test-code-gen test-api-spec clean

lint:
	$(call lint,doc/spec.yaml)

# This doesn't work right now: https://github.com/OpenAPITools/openapi-generator/issues/13038.
test-code-gen:
	$(call test_code_gen,doc/spec.yaml)

clean:
	- pkill aptos-node
	- rm -rf /tmp/aptos_api_client
	- rm -f openapitools.json
	- rm -rf .hypothesis

test-api-spec:
	$(call test_api_spec,spec.yaml)

serve:
	cd doc && python3 -m http.server 8888

define test_api_spec
	- pkill aptos-node
	cargo build -p aptos-node
	./../target/debug/aptos-node --test --lazy &

	curl https://raw.githubusercontent.com/vishnubob/wait-for-it/master/wait-for-it.sh > /tmp/wait-for-it.sh
	chmod +x /tmp/wait-for-it.sh
	/tmp/wait-for-it.sh -t 300 localhost:8080

	schemathesis run --method GET \
		--show-errors-tracebacks \
		--code-sample-style=curl \
		--store-network-log=./../target/schemathesis-network-log.yaml \
		--checks all \
		--base-url http://localhost:8080 \
		http://localhost:8080/$(1)
endef

define lint
	npx @redocly/openapi-cli lint $(1) --skip-rule no-empty-servers
endef

define test_code_gen
	echo '{"generator-cli": {"version": "6.0.1"}}' > openapitools.json # v5.3 has bug, pin the version to 5.2.1
	npx @openapitools/openapi-generator-cli generate -g rust -i $(1) -o /tmp/aptos_api_client --package-name aptos_api_client
	cd /tmp/aptos_api_client && cargo build
endef

.PHONY: test lint test-code-gen test-api-spec clean serve
