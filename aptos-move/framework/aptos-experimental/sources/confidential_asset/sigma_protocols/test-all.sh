#!/bin/bash

FILTER="${1:-sigma}"
TEST_FILTER="$FILTER" time cargo test -- experimental --skip prover
