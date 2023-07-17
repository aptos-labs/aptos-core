# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

test:
	poetry run python -m unittest discover -s aptos_sdk/ -p '*.py' -t ..

test-coverage:
	poetry run python -m coverage run -m unittest discover -s aptos_sdk/ -p '*.py' -t ..
	poetry run python -m coverage report

fmt:
	find ./examples ./aptos_sdk *.py -type f -name "*.py" | xargs poetry run autoflake -i -r --remove-all-unused-imports --remove-unused-variables --ignore-init-module-imports
	poetry run isort aptos_sdk examples
	poetry run black aptos_sdk examples

lint:
	poetry run mypy aptos_sdk
	poetry run flake8 aptos_sdk examples

examples:
	poetry run python -m examples.aptos-token
	poetry run python -m examples.read-aggregator
	poetry run python -m examples.simple-nft
	poetry run python -m examples.simulate-transfer-coin
	poetry run python -m examples.transfer-coin
	poetry run python -m examples.transfer-two-by-two

.PHONY: examples fmt lint test
