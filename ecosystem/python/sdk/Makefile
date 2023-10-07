# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

test:
	poetry run python -m unittest discover -s aptos_sdk/ -p '*.py' -t ..

test-coverage:
	poetry run python -m coverage run -m unittest discover -s aptos_sdk/ -p '*.py' -t ..
	poetry run python -m coverage report

fmt:
	find ./examples ./aptos_sdk . -type f -name "*.py" | xargs poetry run autoflake -i -r --remove-all-unused-imports --remove-unused-variables --ignore-init-module-imports
	poetry run isort aptos_sdk examples
	poetry run black aptos_sdk examples

lint:
	poetry run mypy aptos_sdk examples
	poetry run flake8 aptos_sdk examples

examples:
	poetry run python -m examples.aptos_token
	poetry run python -m examples.read_aggregator
	poetry run python -m examples.simple_nft
	poetry run python -m examples.simple_aptos_token
	poetry run python -m examples.simulate_transfer_coin
	poetry run python -m examples.transfer_coin
	poetry run python -m examples.transfer_two_by_two

examples_cli:
	poetry run python -m unittest -b examples.integration_test

.PHONY: examples fmt lint test
