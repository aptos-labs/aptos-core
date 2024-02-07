#!/bin/bash

for ((x=$1; x<($1+$2); x++))
do
    dir="$3net-$x-$((x+1))m"
    target/release/aptos-comparison-testing \
    --begin-version $((x*1000000)) --limit 1000000 dump https://api.$3net.aptoslabs.com/v1 $dir
    cat $dir/err_log.txt
done