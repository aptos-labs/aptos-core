#!/bin/bash

set -e

scriptdir=$(cd $(dirname $0); pwd -P)

cargo criterion-means --help 2>&1 || { echo "ERROR: Run 'cargo install cargo-criterion-means'"; exit 1; }

repo_root=$(readlink -f $scriptdir/../../../)
#echo "Repo root: $repo_root"

read -p "Delete past benchmark results and re-run? (Otherwise, will use extant data in $repo_root/target/criterion) [y/N]: " ANS

if [ "$ANS" == "y" ]; then
    cd $repo_root/crates/aptos-dkg/

    echo "Cleaning previous chunky_v1 criterion benchmark results..."
    rm -rf $repo_root/target/criterion/pvss_chunky_v1*

    echo "Benchmarking chunky_v1 (with RAYON_NUM_THREADS=1)..."
    RAYON_NUM_THREADS=1 cargo bench --bench pvss -- pvss/chunky_v1/bls12-381

    echo "Cleaning previous chunky_v2 criterion benchmark results..."
    rm -rf $repo_root/target/criterion/pvss_chunky_v2*

    echo "Benchmarking chunky_v2 (with RAYON_NUM_THREADS=1)..."
    RAYON_NUM_THREADS=1 cargo bench --bench pvss -- pvss/chunky_v2/bls12-381

    cd - &>/dev/null
else
    echo "Using existing benchmark data from $repo_root/target/criterion"
    echo "WARNING: Make sure this data was generated with RAYON_NUM_THREADS=1"
fi

cd $repo_root
csv_data=`cargo criterion-means | grep -E '^(Group|pvss_chunky_v1|pvss_chunky_v2)'`

csv_file=`mktemp`
echo "$csv_data" >$csv_file
echo "Wrote CSV file to $csv_file..."

# Change to criterion directory so Python script can find the benchmark folders
cd $repo_root/target/criterion
md_tables=`$scriptdir/print-pvss-markdown-table.py $csv_file`
cd - &>/dev/null

echo "$md_tables"

echo "$md_tables" | pbcopy
echo
echo "Copied to clipboard!"