#!/bin/bash

set -e
set -x

for i in $(seq 0 100); do
  cargo test --release -- test_truncation
done
