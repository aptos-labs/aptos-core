#!/bin/bash

# This assumes you have already poetry installed.
# If not, please follow README.md instructions to install poetry and necessary dependencies.

# Run the necesary linters and formatters for CLI e2e tests
echo "Start linter.sh..."

# Run isort
echo "Running isort..."
poetry run isort .

# Run black
echo "Running black..."
poetry run black .

echo "linter.sh complete!"