import paramiko
import threading
from google.cloud import compute_v1
import google.auth
from google.auth.transport.requests import Request


# This script requires enabling Google API for your Google Cloud project, installing python packages for
# Google Cloud API and authorizing your credentials. See the following tutorial:
# https://developers.google.com/docs/api/quickstart/python

# Global list of VM instances
instances = [
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "run-benchmark-1"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-1"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-2"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-3"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-4"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-5"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-6"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-7"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-8"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-9"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-10"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-11"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-12"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-13"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-14"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-15"},
    {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-16"},
    # Add more instances as needed
]

local_ip_address = {
    "sharding-executor-1": "10.138.0.4",
    "sharding-executor-2": "10.138.0.5",
    "sharding-executor-3": "10.138.0.6",
    "sharding-executor-4": "10.138.0.7",
    "sharding-executor-5": "10.138.0.8",
    "sharding-executor-6": "10.138.0.9",
    "sharding-executor-7": "10.138.0.10",
    "sharding-executor-8": "10.138.0.11",
    "sharding-executor-9": "10.138.0.19",
    "sharding-executor-10": "10.138.0.20",
    "sharding-executor-11": "10.138.0.21",
    "sharding-executor-12": "10.138.0.22",
    "sharding-executor-13": "10.138.0.23",
    "sharding-executor-14": "10.138.0.27",
    "sharding-executor-15": "10.138.0.36",
    "sharding-executor-16": "10.138.0.37",

}

# Global list of commands to be executed on each VM
# commands = [f"cd aptos-core/ && git remote set-url origin https://github.com/aptos-labs/aptos-core && git checkout main && git fetch && git pull && git checkout multi_machine_sharding",]
# commands4shards = [
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
# ]

# commands6shards = [
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 4 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 5 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
# ]

rem_exe_add = "--remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209"
metrics = "PUSH_METRICS_NAMESPACE=jan-benchmark PUSH_METRICS_ENDPOINT=https://gw-c7-2b.cloud.victoriametrics.com/api/v1/import/prometheus PUSH_METRICS_API_TOKEN=06147e32-17de-4d29-989e-6a640ab50f13"
commands8shards = [
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-0.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-1.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-2.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-3.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 4 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-4.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 5 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-5.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 6 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-6.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 7 --num-shards 8 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-7.log",
    #
]

rem_exe_add = "--remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.19:522010 10.138.0.20:522011"
commands10shards = [
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 4 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 5 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 6 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 7 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 8 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 9 --num-shards 10 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 40",
]


commands12shards = [
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 4 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 5 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 6 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 7 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 8 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 9 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 10 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 11 --num-shards 12 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.14:52210 10.138.0.15:52211 10.138.0.16:52212 10.138.0.17:52213 --num-executor-threads 48",
    #
]

rem_exe_add = "--remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 10.138.0.10:52208 10.138.0.11:52209 10.138.0.19:52210 10.138.0.20:52211 10.138.0.21:52212 10.138.0.22:52213 10.138.0.23:52214 10.138.0.27:52215 10.138.0.36:52216 10.138.0.37:52217"
metrics = "PUSH_METRICS_NAMESPACE=jan-benchmark PUSH_METRICS_ENDPOINT=https://gw-c7-2b.cloud.victoriametrics.com/api/v1/import/prometheus PUSH_METRICS_API_TOKEN=06147e32-17de-4d29-989e-6a640ab50f13"
# metrics = ""
commands16shards = [
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-0.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-1.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-2.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-3.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 4 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-4.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 5 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-5.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 6 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-6.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 7 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-7.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 8 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-8.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 9 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-9.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 10 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-10.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 11 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-11.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 12 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-12.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 13 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-13.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 14 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-14.log",
    f"cd aptos-core && {metrics} /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 15 --num-shards 16 --coordinator-address 10.138.0.3:52200 {rem_exe_add} --num-executor-threads 48 > executor-15.log",
]

git_update_command = [
    f"cd aptos-core/ && git remote set-url origin https://github.com/aptos-labs/aptos-core && git checkout main && git fetch && git pull && git checkout multi_machine_sharding_jan_playground && git pull",
]

git_update_command = [
    f"cd aptos-core/ && git checkout multi_machine_sharding_jan_playground && git pull",
]

git_update_command = [
    f"cd aptos-core/ && git checkout multi_machine_sharding && git pull",
]

git_update_command = [
    f"cd aptos-core/ && git checkout multi_machine_sharding_multi_thread_kv_rx_handler && git pull",
]

git_update_command = [
    f"cd aptos-core/ && git pull && git checkout multi_machine_sharding_new_metrics && git pull",
]

def get_external_ip(instance):
    credentials, project = google.auth.default()
    credentials.refresh(Request())
    compute_client = compute_v1.InstancesClient(credentials=credentials)

    instance_details = compute_client.get(
        project=instance['project'],
        zone=instance['zone'],
        instance=instance['instance_name']
    )
    for interface in instance_details.network_interfaces:
        if interface.access_configs:
            return interface.access_configs[0].nat_i_p
    return None

def instance_session(instance, username, private_key_path, close_event, command):
    ip = get_external_ip(instance)
    if not ip:
        print(f"Could not get external IP for {instance['instance_name']}")
        return

    # Execute all commands from the global commands list
    ssh = paramiko.SSHClient()
    ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    try:
        ssh.connect(ip, username=username, key_filename=private_key_path)
        print(f"Connected to {instance['instance_name']} at {ip}")
        stdin, stdout, stderr = ssh.exec_command(f'/bin/bash -c "{command}"', get_pty=True)
        output = stdout.read().decode()
        error = stderr.read().decode()
        print(output)
        print(error)
    except Exception as e:
        return str(e), ""

def run_sessions_on_instances(instances, username, private_key_path):
    close_event = threading.Event()
    threads = []
    i = 0
    for instance in instances:
        thread = threading.Thread(target=instance_session, args=(instance, username, private_key_path, close_event, commands16shards[i]))
        thread.start()
        threads.append(thread)
        i = i + 1

    for thread in threads:
        thread.join()

if __name__ == "__main__":
    ssh_username = "janolkowski"
    private_key_path = "/Users/janolkowski/.ssh/google_compute_engine"

    run_sessions_on_instances(instances, ssh_username, private_key_path)