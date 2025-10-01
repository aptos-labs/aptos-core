#!/bin/bash

set -e
set -x

for i in $(seq 0 100); do
  echo $i
  cargo test
done
