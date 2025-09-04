# Copyright © Velor Foundation
# Parts of the project are originally copyright © Meta Platforms, Inc.
# SPDX-License-Identifier: Apache-2.0

test: clean lint test-code-gen clean

lint:
	$(call lint,doc/spec.yaml)

# This doesn't work right now: https://github.com/OpenAPITools/openapi-generator/issues/13038.
test-code-gen:
	$(call test_code_gen,doc/spec.yaml)

clean:
	- pkill velor-node
	- rm -rf /tmp/velor_api_client
	- rm -f openapitools.json
	- rm -rf .hypothesis

serve:
	cd doc && python3 -m http.server 8888

define lint
	npx @redocly/openapi-cli lint $(1) --skip-rule no-empty-servers
endef

define test_code_gen
	echo '{"generator-cli": {"version": "6.0.1"}}' > openapitools.json # v5.3 has bug, pin the version to 5.2.1
	npx @openapitools/openapi-generator-cli generate -g rust -i $(1) -o /tmp/velor_api_client --package-name velor_api_client
	cd /tmp/velor_api_client && cargo build
endef

.PHONY: test lint test-code-gen test-api-spec clean serve
