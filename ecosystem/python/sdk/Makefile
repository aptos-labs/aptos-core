# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

test:
	- poetry run python -m unittest discover -s aptos_sdk/ -p '*.py' -t ..

fmt:
	- find aptos_sdk -type f -name "*.py" | xargs poetry run autoflake -i -r --remove-all-unused-imports --remove-unused-variables --ignore-init-module-imports
	- find aptos_sdk -type f -name "*.py" | xargs poetry run isort
	- find aptos_sdk -type f -name "*.py" | xargs poetry run black

examples:
	- poetry run python -m examples.coin
	- poetry run python -m examples.token

.PHONY: test fmt examples
