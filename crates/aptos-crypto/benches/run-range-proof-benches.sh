#!/bin/bash

set -e

scriptdir=$(cd $(dirname $0); pwd -P)

repo_root=$(readlink -f $scriptdir/../../../)
#echo "Repo root: $repo_root"

read -p "Delete past benchmark results and re-run? (Otherwise, will use extant data in $repo_root/target/criterion) [y/N]: " ANS

if [ "$ANS" == "y" ]; then
#    echo "Cleaning previous Bulletproof criterion benchmark results..."
#    rm -rf $repo_root/target/criterion/bulletproofs

#    echo "Benchmarking Bulletproofs..."
#    RAYON_NUM_THREADS=1 cargo bench --bench bulletproofs

    echo "Cleaning previous DeKART criterion benchmark results..."
    rm -rf $repo_root/target/criterion/dekart*

    cd $repo_root/crates/aptos-dkg/
    echo "Benchmarking univariate DeKART..."
    RAYON_NUM_THREADS=1 cargo bench --bench range_proof -- dekart-rs/bls12-381
    echo "Benchmarking multivariate DeKART..."
    RAYON_NUM_THREADS=1 cargo bench --bench range_proof -- dekart-multivar/bls12-381
    cd - &>/dev/null
fi

cd $repo_root
csv_data=`cargo criterion-means | grep -E '^(bulletproofs|dekart-rs|dekart-multivar|Group)'`

csv_file=`mktemp`
echo "$csv_data" >$csv_file
echo "Wrote CSV file to $csv_file..."

md_tables=`$scriptdir/print-range-proof-markdown-table.py $csv_file`

echo "$md_tables"

echo "$md_tables" | pbcopy
echo
echo "Copied to clipboard!"
