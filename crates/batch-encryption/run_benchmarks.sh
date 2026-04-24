#!/bin/bash

set -e

if [ $# -lt 1 ]; then
  echo "Usage: run_benchmarks.sh <machine-label> <number of cores [<second number of cores> ..]"
  echo "For example, if you are on a GCP VM of type t2d-standard-60, and want to run benchmarks with 1, 16, and all cores:"
  echo ""
  echo "       run_benchmarks.sh t2d-standard-60 1 16"
  echo ""
  echo "(Note that the default case of all cores is always run.)"

  exit 1
fi

set -x

machine_label=$1
echo $machine_label

export CARGO_NET_GIT_FETCH_WITH_CLI=true

for ncpus in "$@"; do
  # this is a hacky way to have a "default" benchmarking task that's not dependent on ncpus
  if [ $ncpus == $machine_label ]; then
    cargo bench > benchmark_results/$machine_label.all-cpus.txt
  else
    last_cpu=$((ncpus-1))
    ncpus_leading=$(printf "%02d" $ncpus)
    taskset --cpu-list 0-$last_cpu cargo bench > benchmark_results/${machine_label}.${ncpus_leading}-cpus.txt
  fi
done

