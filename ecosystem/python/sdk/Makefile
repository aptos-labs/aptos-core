# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

test:
	- poetry run python -m unittest discover -s aptos_sdk/ -p '*.py' -t ..

fmt:
	- find ./examples ./aptos_sdk *.py -type f -name "*.py" | xargs poetry run autoflake -i -r --remove-all-unused-imports --remove-unused-variables --ignore-init-module-imports
	- poetry run black aptos_sdk examples setup.py
	- poetry run isort aptos_sdk examples setup.py

lint:
	- poetry run mypy aptos_sdk
	- poetry run flake8 aptos_sdk examples setup.py

examples:
	- poetry run python -m examples.transfer-coin
	- poetry run python -m examples.simple-nft
	- poetry run python -m examples.transfer-two-by-two
	- poetry run python -m examples.simulate-transfer-coin

.PHONY: test fmt examples
