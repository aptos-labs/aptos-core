#!/bin/bash

# cd to repo root
cd "$(git rev-parse --show-toplevel)"

pnpm install
pnpm test docker/__tests__