---
title: "Self-Hosted Indexer API"
---

# Self-Hosted Indexer API

This guide will walk you through setting up a self-hosted Indexer API.

:::caution
Currently this guide only explains how to run processor part of the Indexer API. By the end of this guide you will have a running processor that consumes transactions from the Transaction Stream Service, parses them, and stores them in the database. Unfortunately this guide does not explain how to attach an API to this system right now.
:::

## Prerequisites

- A running PostgreSQL instance is required, with a valid user and database. In this example we call the user `postgres` and the database `indexer`.
- If you wish to use Docker, you must have Docker installed. [Installation Guide](https://docs.docker.com/get-docker/).


## Configuration
To run the service we need to define a config file. We will start with this template:

```yaml
health_check_port: 8084
server_config:
  processor_name: default_processor
  postgres_connection_string: postgresql://postgres:@localhost:5432/indexer
  indexer_grpc_data_service_address: 127.0.0.1:50051
  indexer_grpc_http2_ping_interval_in_secs: 60
  indexer_grpc_http2_ping_timeout_in_secs: 10
  auth_token: AUTH_TOKEN
```

From here you will likely want to change the values of some of these fields. Let's go through some of them.

### `processor_name`
:::info
A single instance of the service only runs a single processor. If you want to run multiple processors, you must run multiple instances of the service. In this case, it is up to you whether to use the same database or not.
:::

This is the processor you want to run. You can see what processors are available [here](https://github.com/aptos-labs/aptos-indexer-processors/blob/main/rust/processor/src/processors/mod.rs#L23). Some examples:
- `coin_processor`
- `ans_processor`
- `token_v2_processor`

### `postgres_connection_string`
This is the connection string to your PostgreSQL database. It should be in the format `postgresql://<username>:<password>@<host>:<port>/<database>`.

### `indexer_grpc_data_service_address`
This is the URL for the Transaction Stream Service. If you are using the Labs-Hosted instance you can find the URLs for each network at [this page](../txn-stream/labs-hosted). Make sure to select the correct URL for the network you want to index. If you are running this service locally the value should be `127.0.0.1:50051`.

### `auth_token`
This is the auth token used to connect to the Transaction Stream Service. If you are using the Labs-Hosted instance you can use the API Gateway to get an auth token. Learn more at [this page](/indexer/txn-stream/labs-hosted).

## Run with source code
Clone the repo:
```
# SSH
git clone git@github.com:aptos-labs/aptos-indexer-processors.git

# HTTPS
git clone https://github.com/aptos-labs/aptos-indexer-processors.git
```

Navigate to the directory for the service:
```
cd aptos-indexer-processors
cd rust/processor
```

Run the service:
```
cargo run --release -- -c config.yaml
```

## Run with Docker
<!--
This doesn't actually work this very moment because:

1. We don't yet publish the image as indexer-processor-rust
2. We don't tag it as latest.

We'll do that soon though: https://aptos-org.slack.com/archives/C04PRP1K1FZ/p1692732083583659
-->

To run the service with Docker, use the following command:
```
docker run -it --network host --mount type=bind,source=/tmp/config.yaml,target=/config.yaml aptoslabs/indexer-processor-rust -c /config.yaml
```

This command binds the container to the host network and mounts the config file from the host into the container. This specific invocation assumes that your config file in the host is at `/tmp/config.yaml`.
