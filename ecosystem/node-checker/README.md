# Aptos Node Health Checker
The Aptos Node Health Checker (NHC) is the reference implementation of a node health checker for Validator Nodes (Validators), Validator FullNodes (VFNs), and Public FullNodes (PFNs). The node health checker aims to serve 3 major user types:
- **AIT Registration**: As part of sign up for the Aptos Incentivized Testnets (AIT), we request that users demonstrate that they can run a ValidatorNode successfully. We use this tool to encode precisely what that means.
- **Operator Support**: As node operators, you will want to know whether your node is running correctly. This service can help you figure that out. While we host our own instances of this service, we encourage node operators to run their own instances. You may choose to either run a publicly available NHC or run it as a sidecar, where it only works against your own node.
- **Continuous Evaluation**: As part of the AITs, Aptos Labs needs a tool to help confirm that participants are running their nodes in a way that meets our criteria. We run this tool continuously throughout each AIT to help us evaluate this.

In this README we describe how to run NHC for the **Operator Support** use case. NHC can reasonably be run both as an external tool as well as a sidecar process for this use case. Both are described below. For more information on how NHC works, please see [How NHC works](#how-nhc-works) below.

## tl;dr
While we highly recommend you read this whole README, you can get NHC working in a basic form by doing the following. This baseline configuration is for a devnet fullnode.

Get a baseline configuration:
```
cd /tmp/nhc && wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/ecosystem/node-checker/configurations/devnet_fullnode.yaml
```

Run NHC:
```
docker run -v /tmp/nhc:/nhc -t aptoslabs/node-checker:nightly /usr/local/bin/aptos-node-checker server run --baseline-node-config-paths /nhc/devnet_fullnode.yaml
```

Hit it with a request:
```
curl 'http://localhost:20121/check_node?node_url=http://mynode.mysite.com&baseline_configuration_name=devnet_fullnode'
```

## How NHC works
Before running NHC, it is important to know at a high level how NHC works. In short, NHC runs as a service that you send http requests to in order to run a set of validations against your node. A single NHC instance can be configured to test multiple different node configurations, for example:

- Validator Node running in single node testnet.
- Public FullNode connected to devnet.
- Validator Node connected to testnet, e.g. as part of an Aptos Incentivized Testnet.

In all cases, validations are performed compared to a baseline. For example, for the second configuration above, the baseline node might be a node run by the Aptos team that demonstrates optimal performance / participation characteristics. The configuration describes where to find this node (URL + port), what evaluators (e.g. metrics checks, TPS tests, API validations, etc.) NHC should runn, what parameters to use for those evaluators, what name the configuration has, and so on. Your node will be compared to this baseline node.

When you send requests to NHC, you include which configuration you want to validate your node against. This means a request to NHC might look like this:
```
curl 'http://nhc.aptoslabs.com/check_node?node_url=http://myfullnode.mysite.com&baseline_configuration_name=devnet_fullnode'
```

## Getting configurations ready
In order to run NHC, you must have baseline configurations that it can use. You have two options here:

### Start from a pre-existing configuration
In [./configuration_examples](./configuration_examples) you can find configurations that work for each of the use cases above and more.

You might want to setup configurations in your host system like this:
```
mkdir /etc/nhc
cd /etc/nhc
configs=(single_node_validator devnet_fullnode ait2_validator); for c in ${configs[@]}; do wget https://raw.githubusercontent.com/aptos-labs/aptos-core/main/ecosystem/node-checker/configurations/$c.yaml; done
```

These configurations are not quite ready to be used as they are, you will need to modify certain fields, such as the node address or evaluator set used. The best way to iterate on this is to just try to run NHC with the configuration and see what it says on startup.

### Generate your own configurations
To generate your own configurations, you must first get your hands on NHC. Follow one of the guides below for that. Assuming you're using NHC from an image, you could generate a configuration with a command like this:
```
docker run -it aptoslabs/node-checker:nightly /usr/local/bin/aptos-node-checker configuration create --url 'http://baseline-fullnode.aptoslabs.com' --configuration-name devnet_fullnode --configuration-name-pretty "Devnet FullNode" --evaluators network_minimum_peers api_latency --api-port 80 > /etc/nhc/devnet_fullnode.yaml
```

This command just specifies the bare minimum for a baseline configuration, you can tune each evaluator as you see fit. For more guidance on this, try passing `-h` to the above command and seeing all the flags you can work with.

### Getting necessary files
For some NHC configurations, you will need accompanying files, e.g. `mint.key` to use for running a TPS test against a validator. You should make sure those are also avilable to NHC, either on disk or mounted into your container. NHC will expect them on startup at a path determined by the baseline configuration.

## Running NHC: Docker
Assuming you've followed the configuration guide above, you can mount and use the configurations and then run the server with a command like this:
```
docker run -v /etc/nhc:/etc/nhc -p 20121:20121 -t aptoslabs/node-checker:nightly /usr/local/bin/aptos-node-checker server run --baseline-node-config-paths /etc/nhc/ait2_validator.yaml /etc/nhc/devnet_fullnode.yaml
```

You may want to include other env vars such as `RUST_LOG=info`. As you can see, by default NHC runs on port 20121. Make sure to publish it from the container like in the above command and ensure the port is open on your host. You may change the port NHC runs on with `--listen-port`.

## Running NHC: Source
First, check out the source:
```
git clone git@github.com:aptos-labs/aptos-core.git
cd aptos-core
```

Depending on your setup, you may want to check out a particular branch, to ensure NHC is compatible with your node, e.g. `git checkout --track devnet`.

From here, assuming you have followed the above configuration guide, you can run NHC:
```
cargo run --release -- server run --baseline-node-config-paths /etc/nhc/ait2_validator.yaml /etc/nhc/devnet_fullnode.yaml
```

## Running NHC: Terraform / Helm
Down the line we will have easier pre-packaged configs in which you only need to specify key pieces of the configuration. Coming soon!

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

There are more options than these, e.g. around which ports to use. Pass `-h` to see more options.

---

## Generating the OpenAPI specs
To generate the OpenAPI specs, run the following commands:
```
cargo run -- server generate-openapi -f yaml > openapi.yaml
cargo run -- server generate-openapi -f json > openapi.json
```

You can also hit the `spec_yaml` and `spec_json` endpoints of the running service.

## Developing
To develop this app, you should first run two nodes of the same type. See [this wiki](https://aptos.dev/tutorials/full-node/run-a-fullnode) for guidance on how to do this. You may also target a known existing FullNode with its metrics port open.

The below command assumes we have a fullnode running locally, the target node (the node under investigation), and another running on a machine in our network, the baseline node (the node we compare the target to):
```
cargo run -- --baseline-node-url 'http://192.168.86.2' --target-node-url http://localhost --evaluators state_sync_version --allow-preconfigured-test-node-only
```
This runs NHC in sidecar mode, where only the `/check_preconfigured_node` endpoint can be called, which will target the node running on localhost.

Once the service is running, you can query it like this:
```
$ curl -s localhost:20121/check_preconfigured_node | jq .
{
  "evaluations": [
    {
      "headline": "State sync version is within tolerance",
      "score": 100,
      "explanation": "Successfully pulled metrics from target node twice, saw the version was progressing, and saw that it is within tolerance of the baseline node. Target version: 1882004. Baseline version: 549003. Tolerance: 1000"
    }
  ],
  "summary_score": 100,
  "summary_explanation": "100: Awesome!"
}
```
