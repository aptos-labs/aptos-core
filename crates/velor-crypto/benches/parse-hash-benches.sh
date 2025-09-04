#!/bin/bash

set -e

scriptdir=$(cd $(dirname $0); pwd -P)

repo_root=$(readlink -f $scriptdir/../../../)

#echo "Repo root: $repo_root"

critdir=$repo_root/target/criterion/hash

if [ $# -ne 2 ]; then
    echo "Usage: $0 <hash-func-name> <outfile>"
    echo
    echo "<hash-func-name> can be:"
    echo
    echo "  SHA2-256"
    echo "  SHA2-512"
    echo "  SHA3-256"
    echo "  hash_to_bls12381_g1"
    echo "  hash_to_bls12381_g2"
    echo "  Keccak-256"
    
    exit 1
fi

hash_func=$1; shift
outfile=$1

if [ ! -f $critdir/$hash_func/0/new/estimates.json ]; then
    echo "No benchmark results for $hash_func. Be sure to run 'cargo bench' first..."
    exit 1
fi

ns="0 1 2 4 8 16 32 64 128 256 512 1024" # 2048 4096 8192 16384 32768 65536"

for n in $ns; do
    mean=`cat $critdir/$hash_func/$n/new/estimates.json | jq .slope.point_estimate`
    if [ "$mean" = "null" ]; then
        #echo "No .slope.point_estimate, using .mean.point_estimate" 1>&2

        mean=`cat $critdir/$hash_func/$n/new/estimates.json | jq .mean.point_estimate`
    fi
    #echo "$n -> $mean"
    echo "$hash_func,$n,$mean" >>$outfile
done
