---
title: "Node Health Checker"
slug: "index"
---

# Node Health Checker

The Aptos Node Health Checker (NHC) service can be used to check the health of the following Aptos node types:

- Validator nodes.
- Validator fullnodes, and
- Public fullnodes.

If you are a node operator, use the NHC service to check if your node is running correctly. The NHC service evaluates your node's health by comparing against a baseline node configuration, and outputs the evaluation results.

:::tip Node health check for AIT
If you are participating in the consensus in an Aptos network, you can use the NHC service to verify that your validator node is running correctly. The Aptos team uses this service continuously to check your node's health.
:::

This document describes how to run NHC when you are operating a node.

## Quickstart

Before you get into the details of how NHC works, you can run the below steps to start the NHC service and send it a request. This quickstart uses a baseline configuration for a devnet fullnode, i.e., it will evaluate your node against a devnet fullnode that is configured with the baseline configuration YAML.

**Important**: If your local node is not a devnet fullnode, e.g. it is a single testnet node for AIT3 registration, you must use a different configuration file. See [the configuration examples in aptos-core](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/node-checker/configuration_examples) for other such example configs.

### Step 1: Download the baseline configuration YAML

Download a baseline configuration YAML file for a devnet fullnode. The below command will download the `devnet_fullnode.yaml` configuration file:

```
mkdir /tmp/nhc && cd /tmp/nhc && wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/ecosystem/node-checker/configuration_examples/devnet_fullnode.yaml
```

### Step 2: Start the NHC service

Start the NHC service by providing the above-downloaded `devnet_fullnode.yaml` baseline configuration YAML file:

```
docker run -v /tmp/nhc:/nhc -p 20121:20121 -t aptoslabs/node-checker:nightly /usr/local/bin/aptos-node-checker server run --baseline-node-config-paths /nhc/devnet_fullnode.yaml
```

### Step 3: Send a request to NHC service

Finally, send a request to the NHC service you started above. The following command runs health checks of your node that is at `node_url=http://mynode.mysite.com` and compares these results with the downloaded baseline configuration `devnet_fullnode`:

```
curl 'http://localhost:20121/check_node?node_url=http://mynode.mysite.com&api_port=80&baseline_configuration_name=devnet_fullnode'
```

You will see output similar to this:

```
{
  "evaluation_results": [
    {
      "headline": "Chain ID reported by baseline and target match",
      "score": 100,
      "explanation": "The node under investigation reported the same Chain ID 18 as is reported by the baseline node",
      "evaluator_name": "node_identity",
      "category": "api",
      "links": []
    },
    {
      "headline": "Role Type reported by baseline and target match",
      "score": 100,
      "explanation": "The node under investigation reported the same Role Type full_node as is reported by the baseline node",
      "evaluator_name": "node_identity",
      "category": "api",
      "links": []
    },
    {
      "headline": "Target node produced valid recent transaction",
      "score": 100,
      "explanation": "We were able to pull the same transaction (version: 3238616) from both your node and the baseline node. Great! This implies that your node is keeping up with other nodes in the network.",
      "evaluator_name": "transaction_availability",
      "category": "api",
      "links": []
    }
  ],
  "summary_score": 100,
  "summary_explanation": "100: Awesome!"
}
```

## How NHC works

The NHC runs as a service. When you want to run a health check of your node, you send the HTTP requests to this service.

A single NHC instance can be configured to check the health of multiple node configurations, each of different type, for example:

- A validator node running in a single node testnet.
- A public fullnode connected to the Aptos devnet.
- A validator node connected to a testnet, for example, as part of an Aptos Incentivized Testnet.

The NHC service can reasonably be run both as an external tool as well as a sidecar process for the operator use case. Both are described in this documentation.

### Baseline configuration

In all the above cases, a baseline node is used to compare your node's health. For example, for a public fullnode connected to the Aptos devnet, the baseline node might be a node run by the Aptos team and this node demonstrates optimal performance and participation characteristics.

You will download the baseline configuration YAML before running the NHC service for your node. The baseline node's configuration YAML describes where to find this baseline node (URL + port), what evaluators (e.g. metrics checks, TPS tests, API validations, etc.) the NHC service should run, what parameters the NHC should use for those evaluators, what name the configuration has, and so on. See some [example baseline configuration YAML files here](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/node-checker/configuration_examples).

When you send requests to the NHC service, you must include a baseline configuration. For example, a request to NHC to use `devnet_fullnode` as the baseline configuration will look like this:

```
curl 'http://nhc.aptoslabs.com/check_node?node_url=http://myfullnode.mysite.com&baseline_configuration_name=devnet_fullnode'
```

### Getting baseline configurations ready

In order to run the NHC service, you must have a baseline configuration that the service can use. You have two options here:

#### Configure a pre-existing YAML

You can find a few [example baseline configuration YAML files here](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/node-checker/configuration_examples) that work for each of the above use cases and more.

Next, download these configuration YAML files into the `/etc/nhc` folder in your host system. For example:

```
mkdir /etc/nhc
cd /etc/nhc
configs=(single_node_validator devnet_fullnode ait2_validator); for c in ${configs[@]}; do wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/ecosystem/node-checker/configurations/$c.yaml; done
```

These configurations are not quite ready to be used as they are. You will need to modify certain fields, such as the baseline node address or evaluator set (`evaluators` and `evaluator_args` in the YAML) used. The best way to iterate on this is to run the NHC with a downloaded baseline configuration and see what it says on startup.

#### Generate your own baseline configuration YAML

To generate your own baseline configuration, you must first run the NHC service with `create` option. The below command shows how to create a baseline configuration YAML by running the NHC service using Docker:

```
docker run -it aptoslabs/node-checker:nightly /usr/local/bin/aptos-node-checker configuration create --url 'http://baseline-fullnode.aptoslabs.com' --configuration-name devnet_fullnode --configuration-name-pretty "Devnet fullnode" --evaluators network_minimum_peers api_latency --api-port 80 > /etc/nhc/devnet_fullnode.yaml
```

The above command specifies the bare minimum for a baseline configuration. You can tune each evaluator as you see fit. See the fields `evaluators` and `evaluator_args` in the YAML. For more guidance on this, pass the `-h` flag to the above command to see all the flags you can work with.

### Required files

For some NHC configurations, you will need accompanying files, e.g. `mint.key` to use for running a TPS test against a validator. You should make sure these files are also available to NHC, either on disk or mounted into your container. NHC expects them on startup at a path specified in the baseline configuration YAML.

## Running NHC: Docker

:::tip
While the Aptos team hosts our own instances of this service, we encourage node operators to run their own instances. You may choose to either run a publicly available NHC or run it as a sidecar, where it only works against your own node.
:::

When you are ready with baseline configuration YAML and the required files, you can run the NHC server with a command like this, for example, with Docker:

```
docker run -v /etc/nhc:/etc/nhc -p 20121:20121 -t aptoslabs/node-checker:nightly /usr/local/bin/aptos-node-checker server run --baseline-node-config-paths /etc/nhc/ait3_fullnode.yaml /etc/nhc/devnet_fullnode.yaml
```

:::tip

You may want to include other environment variables such as `RUST_LOG=info`. As you can see, by default NHC runs on port 20121. Make sure to publish it from the container, as shown in the above command, and ensure the port is open on your host. You may change the port NHC runs on with `--listen-port`.
:::

## Running NHC: Source

First, check out the source:

```
git clone git@github.com:aptos-labs/aptos-core.git
cd aptos-core
```

Depending on your setup, you may want to check out a particular branch, to ensure NHC is compatible with your node, e.g. `git checkout --track devnet`.

Run NHC:

```
cargo run --release -- server run --baseline-node-config-paths /etc/nhc/ait2_validator.yaml /etc/nhc/devnet_fullnode.yaml
```


## Running NHC as a sidecar

When you run NHC as a sidecar, you preconfigure a node that NHC should use as the node under investigation by default:

```
--target-node-url http://localhost
```

Running NHC as a sidecar can be handy when you want to close the API / metrics ports on your machine to the public internet, but would still like to run NHC to validate the setup of your node.

If you want, you can even restrict NHC to test only that node:

```
--allow-preconfigured-test-node-only
```

With this flag, the `/check_node` endpoint will always return 400s, you must instead use `/check_preconfigured_node`.

Once you have configured your NHC instance in sidecar mode, you can send requests that omit the target node address.

```
curl 'http://nhc.aptoslabs.com/check_preconfigured_node?baseline_configuration_name=devnet_fullnode'
```

There are more options available for which ports to use. Pass `-h` to see more options.

## Generating the OpenAPI specs

To generate the OpenAPI specs, run the following commands:

```
cargo run -- server generate-openapi -f yaml > doc/spec.yaml
cargo run -- server generate-openapi -f json > doc/spec.json
```

You can also hit the `/spec.yaml` and `/spec.json` endpoints of the running service.

