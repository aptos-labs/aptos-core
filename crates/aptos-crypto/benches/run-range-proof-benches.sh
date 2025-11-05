#!/bin/bash

set -e

scriptdir=$(cd $(dirname $0); pwd -P)

repo_root=$(readlink -f $scriptdir/../../../)
#echo "Repo root: $repo_root"

echo "Cleaning previous Bulletproof criterion benchmark results..."
rm -r $repo_root/target/criterion/bulletproofs

echo "Benchmarking Bulletproofs..."
RAYON_NUM_THREADS=1 cargo bench -- bulletproofs

echo "Cleaning previous DeKART criterion benchmark results..."
rm -r $repo_root/target/criterion/dekart*

echo "Benchmarking DeKART..."
cd $repo_root/crates/aptos-dkg/
RAYON_NUM_THREADS=1 cargo bench -- dekart-rs/bls12-381
cd - &>/dev/null

cd $repo_root
csv_data=`cargo criterion-means | grep -E '^(bulletproofs|dekart-rs|Group)'`

csv_file=$scriptdir/range-proofs.csv
echo "Wrote CSV file to $csv_file..."
echo "$csv_data" >$csv_file

prompt=`gsed -n '/PROMPT STARTS HERE/,/PROMPT ENDS HERE/{/PROMPT STARTS HERE/d;/PROMPT ENDS HERE/d;p}' $scriptdir/README.md`

echo "Copied the ChatGPT prompt" # and the CSV data to the clipboard!"
#echo -e "$prompt\n\nCSV data follows below together with named columns...\n\n$csv_data" | pbcopy
echo -e "$prompt" | pbcopy
