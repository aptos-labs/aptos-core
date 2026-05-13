#!/bin/bash

set -e

scriptdir=$(cd $(dirname $0); pwd -P)

# Parse flags
WITH_BULLETPROOFS=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -b|--with-bulletproofs) WITH_BULLETPROOFS=true; shift ;;
        *) echo "Unknown option: $1"; echo "Usage: $0 [-b|--with-bulletproofs]"; exit 1 ;;
    esac
done

cargo criterion-means --help 2&>1 || { echo "ERROR: Run 'cargo install cargo-criterion-means'"; exit 1; }

repo_root=$(readlink -f $scriptdir/../../../)
#echo "Repo root: $repo_root"

read -p "Delete past benchmark results and re-run? (Otherwise, will use extant data in $repo_root/target/criterion) [y/N]: " ANS

if [ "$ANS" == "y" ]; then
    if [ "$WITH_BULLETPROOFS" == "true" ]; then
        echo "Cleaning previous Bulletproof criterion benchmark results..."
        rm -rf $repo_root/target/criterion/bulletproofs

        echo "Benchmarking Bulletproofs..."
        RAYON_NUM_THREADS=1 cargo bench --bench bulletproofs
    fi

    echo "Cleaning previous DeKART criterion benchmark results..."
    rm -rf $repo_root/target/criterion/dekart*

    cd $repo_root/crates/aptos-dkg/
    echo "Benchmarking univariate DeKART..."
    RAYON_NUM_THREADS=1 cargo bench --bench range_proof -- dekart-rs/bls12-381
#    echo "Benchmarking multivariate DeKART..."
#    RAYON_NUM_THREADS=1 cargo bench --bench range_proof -- dekart-multivar/bls12-381
    cd - &>/dev/null
fi

cd $repo_root
if [ "$WITH_BULLETPROOFS" == "true" ]; then
    csv_data=`cargo criterion-means | grep -E '^(bulletproofs|dekart-rs|Group)'`
else
    csv_data=`cargo criterion-means | grep -E '^(dekart-rs|Group)'`
fi
#csv_data=`cargo criterion-means | grep -E '^(bulletproofs|dekart-rs|dekart-multivar|Group)'`

csv_file=`mktemp`
echo "$csv_data" >$csv_file
echo "Wrote CSV file to $csv_file..."

# Change to criterion directory so Python script can find benchmark folders (for proof_size from benchmark.json)
cd $repo_root/target/criterion
md_tables=`$scriptdir/print-range-proof-markdown-table.py $csv_file`
cd - &>/dev/null

echo "$md_tables"

echo "$md_tables" | pbcopy
echo
echo "Copied to clipboard!"
