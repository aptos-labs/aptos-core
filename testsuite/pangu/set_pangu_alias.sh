#!/bin/bash

#
# SET ALIASES FOR PANGU
#
# -Lets you use just pangu in your terminal instead of "poetry run python pangu.py"
# -Lets you format your code with "pfmt" instead of "poetry run poe fmt"
# -Lets you lint your code with "plint" instead of "poetry run poe lint"
# -Lets you run tests with "ptest" instead of "poetry run poe test"
#

# Run this script with:
# source ./set_pangu_alias.sh

# To remove the alias, run:
# unalias pangu


# Set alias for pangu
alias pangu="poetry run python pangu.py"

# Set alias for formatting code
alias pfmt="poetry run poe fmt"

# Set alias for linting code
alias plint="poetry run poe lint"

# Set alias for running tests
alias ptest="poetry run python -m unittest pangu/**/*test.py"

