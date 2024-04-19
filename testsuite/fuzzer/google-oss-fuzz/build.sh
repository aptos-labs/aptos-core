#!/bin/bash -eu

cd testsuite/fuzzer
bash fuzz.sh build-oss-fuzz $OUT
