<a href="https://aptos.dev">
	<img width="100%" src="./.assets/aptos_banner.png" alt="Aptos Banner" />
</a>

---

# Horizontally Sharding Executor Benchmark
This repo contains the benchmarking code for the horizontally sharded executor. The benchmarking code is written in Rust and requires one coordinator node and multiple shards nodes.

It evaluates the performance of the horizontally sharded executor by measuring the time taken to distribute blocks of transactions to shards for execution and collecting back the results.

## Pre-requisites
One coordinator node and multiple shards nodes (depending on the desired degree of scale-out) are needed to run the benchmark. And the nodes must be on the same data center.

## Specs in our experiments
We spun up the nodes on a GCP data center. The specs are for the nodes are
* Executor Shard Nodes: T2d-60 with 32 Gbps bandwidth, 60 vCPUs (physical cores) running at 2.45 GHz on AMD Milan, 240 GB RAM, and 2 TB SSD persistent disks.
* Coordinator Node: A NUMA machine with 360 vCPUs (180 vCPUs on a single NUMA node used), 100 Gbps bandwidth, AMD Genoa processor, 708 GB RAM, and 2 TB SSD persistent disks.

## Setup
* Decide on the degree of scale-out <NUM_SHARDS> you want to test.
* Clone repo on coordinator and all executor shard nodes
```
  git clone https://github.com/aptos-labs/aptos-core.git && cd aptos-core/ && ./scripts/dev_setup.sh -b > dev_setup.log
```
* On the coordinator, create sufficient user accounts needed for the benchmark
```
  cargo run --profile performance -p aptos-executor-benchmark -- --enable-storage-sharding create-db --data-dir ~/workspace/db_dirs/db/ --num-accounts 2000000
```
* On each of shard nodes, run the following command
```
  cargo run --profile performance -p aptos-executor-service --manifest-path ./execution/executor-service/Cargo.toml -- --shard-id <id> --num-shards NUM_SHARDS --coordinator-address <coord_ip>:<coord_port> --remote-executor-addresses <shard_0_ip>:<shard_0_port> <shard_1_ip>:<shard_1_port> <shard_NUM_SHARDS-1_ip>:<shard_NUM_SHARDS-1_port> --num-executor-threads 48 > > executor-{id}.log
```
  Shard ids start from 0 and go to num_shards - 1. For example, for 2 shards, the shard ids are 0 and 1.
* On the coordinator node, run the relevant benchmark command.
  * Foundational txns workload:
  ```
     taskset -c 0-89,180-269 cargo run --profile performance -p aptos-executor-benchmark -- --block-size 500000 --enable-storage-sharding --foundational-txns --num-executor-shards NUM_SHARDS --connected-tx-grps 500000 --partitioner-version v3-fanout --remote-executor-addresses <shard_0_ip>:<shard_0_port> <shard_1_ip>:<shard_1_port> <shard_NUM_SHARDS-1_ip>:<shard_NUM_SHARDS-1_port> --coordinator-address <coord_ip>:<coord_port> --generate-then-execute --split-stages run-executor --data-dir ~/workspace/db_dirs/db --checkpoint-dir ~/workspace/db_dirs/chk --blocks 50 --main-signer-accounts 500001
  ```
  * Multi DApp workload:
  ```
     taskset -c 0-89,180-269 cargo run --profile performance -p aptos-executor-benchmark -- --block-size 500000 --enable-storage-sharding --num-executor-shards NUM_SHARDS --partitioner-version v3-fanout --fanout-num-iterations 40 --num-clusters 400 --num-resource-addresses-per-cluster 5 --cluster-size-relative-std-dev 0.05 --mean-txns-per-user 5 --txns-per-user-relative-std-dev 0.5 --fraction-of-external-txns 0.001 --remote-executor-addresses <shard_0_ip>:<shard_0_port> <shard_1_ip>:<shard_1_port> <shard_NUM_SHARDS-1_ip>:<shard_NUM_SHARDS-1_port> --coordinator-address <coord_ip>:<coord_port> --generate-then-execute --split-stages run-executor --data-dir ~/workspace/db_dirs/db --checkpoint-dir ~/workspace/db_dirs/chk --blocks 50 --main-signer-accounts 1000001
  ```
  
## Results
