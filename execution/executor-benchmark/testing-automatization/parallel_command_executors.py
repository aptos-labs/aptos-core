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
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-6"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-7"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-8"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-9"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-10"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-11"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-12"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-13"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-14"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-15"},
    # {"project": "aptos-jan-olkowski-playground", "zone": "us-west1-b", "instance_name": "sharding-executor-16"},
    # Add more instances as needed
]

# Global list of commands to be executed on each VM
# commands = [f"cd aptos-core/ && git remote set-url origin https://github.com/aptos-labs/aptos-core && git checkout main && git fetch && git pull && git checkout multi_machine_sharding",]
commands4shards = [
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
    f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 4 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 --num-executor-threads 48",
]

# commands6shards = [
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 0 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 1 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 2 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 3 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 4 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
#     f"cd aptos-core && /home/janolkowski/.cargo/bin/cargo run --profile performance -p aptos-executor-service --manifest-path /home/janolkowski/aptos-core/execution/executor-service/Cargo.toml -- --shard-id 5 --num-shards 6 --coordinator-address 10.138.0.3:52200 --remote-executor-addresses 10.138.0.4:52202 10.138.0.5:52203 10.138.0.6:52204 10.138.0.7:52205 10.138.0.8:52206 10.138.0.9:52207 --num-executor-threads 48",
# ]

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
        thread = threading.Thread(target=instance_session, args=(instance, username, private_key_path, close_event, commands4shards[i]))
        thread.start()
        threads.append(thread)
        i = i + 1

    for thread in threads:
        thread.join()

if __name__ == "__main__":
    ssh_username = "janolkowski"
    private_key_path = "/Users/janolkowski/.ssh/google_compute_engine"

    run_sessions_on_instances(instances, ssh_username, private_key_path)