#!/bin/bash

set -e
set -x

for i in $(seq 0 10); do
  echo $i
  cargo test
done

for i in $(seq 0 10); do
  echo $i
  cargo test --release
done
