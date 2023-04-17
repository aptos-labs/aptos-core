#!/bin/bash

# Run a Python binary using Poetry.

exec poetry -C "$(dirname ${0})" run python3 "${0/.sh/.py}" "${@}"
