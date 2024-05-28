#!/bin/bash

function get_error() {
  find $1 -name compile.log | while read f; do
    grep "error\[E" $f
  done | sort | uniq
}

rm -rf output
cargo run --bin generator -- -o output -s 1234 -p -n $1

for p in output/*; do
  if [ -d "$p" ]; then
    echo "Checking $p"
    (
        cd "$p"
        aptos move compile > compile.log 2>&1
        if [ $? -ne 0 ]; then
          echo "Compile $p: failed"
          get_error .
        else
          echo "Compile $p: success"
        fi
    )
  fi
done

echo
echo "Errors are:"
get_error output
