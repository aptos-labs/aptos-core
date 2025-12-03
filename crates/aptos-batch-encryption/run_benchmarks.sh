#!/bin/bash

set -x
set -e

if [ $# -lt 1 ]; then
  echo "Usage run_benchmarks.sh <machine-label>"
  exit 1
fi

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

