#!/bin/bash

set -e

scriptdir=$(cd $(dirname $0); pwd -P)

repo_root=$(readlink -f $scriptdir/../../../)

#echo "Repo root: $repo_root"

if [ $# -ne 2 ]; then
    echo "Usage: $0 <group-name> <bench-name>"
    echo
    echo "<group-name> can be"
    echo "  bls12381"
    echo "  hash"
    exit 1
fi

group_name=$1; shift
bench_name=$1

critdir=$repo_root/target/criterion/$group_name/$bench_name

mean=`cat $critdir/new/estimates.json | jq .slope.point_estimate`
if [ "$mean" = "null" ]; then
    echo "No .slope.point_estimate, using .mean.point_estimate" 1>&2

    mean=`cat $critdir/$hash_func/$n/new/estimates.json | jq .mean.point_estimate`
fi

echo "$mean"
