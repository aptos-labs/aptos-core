---
title: "Run a Local Development Network"
---

# Run a Local Development Network

You can run the Aptos network locally. This local network will not be connected to any production Aptos network (e.g. mainnet), it will run on your local machine, independent of other Aptos networks. Building against a local network has a few advantages:
- **No ratelimits:** Hosted services (including the Node API, Indexer API, and faucet) are generally subject to ratelimits. Local development networks have no ratelimits.
- **Reproducibility:** When using a production network you might have to repeatedly make new accounts or rename Move modules to avoid incompatibility issues. With a local network you can just choose to start from scratch.
- **High availability:** The Aptos devnet and testnet networks are occasionally upgraded, during which time they can be unavailable. The internet can also be unreliable sometimes. Local development networks are always available, even if you have no internet access.

## Prerequisites
In order to run a local development network you must have the following installed:
- Aptos CLI: [Installation Guide](../tools/aptos-cli/install-cli/index.md).
- Docker: [Installation Guide](https://docs.docker.com/get-docker/).

:::tip
If you do not want to run an [Indexer API](../indexer/api/index.md) as part of your local network (`--with-indexer-api`) you do not need to install Docker. Note that without the Indexer API your local network will be incomplete compared to a production network. Many features in the downstream tooling will not work as expected / at all without this API available.
:::

## Run a local network

You can run a local network using the following Aptos CLI command:
```bash
aptos node run-local-testnet --with-indexer-api
```

**Note:** Despite the name (`local-testnet`), this has nothing to with the Aptos testnet, it will run a network entirely local to your machine.

You should expect to see output similar to this:
```
Readiness endpoint: http://0.0.0.0:8070/

Indexer API is starting, please wait...
Node API is starting, please wait...
Transaction stream is starting, please wait...
Postgres is starting, please wait...
Faucet is starting, please wait...

Completed generating configuration:
        Log file: "/Users/dport/.aptos/testnet/validator.log"
        Test dir: "/Users/dport/.aptos/testnet"
        Aptos root key path: "/Users/dport/.aptos/testnet/mint.key"
        Waypoint: 0:397412c0f96b10fa3daa24bfda962671c3c3ae484e2d67ed60534750e2311f3d
        ChainId: 4
        REST API endpoint: http://0.0.0.0:8080
        Metrics endpoint: http://0.0.0.0:9101/metrics
        Aptosnet fullnode network endpoint: /ip4/0.0.0.0/tcp/6181
        Indexer gRPC node stream endpoint: 0.0.0.0:50051

Aptos is running, press ctrl-c to exit

Node API is ready. Endpoint: http://0.0.0.0:8080/
Postgres is ready. Endpoint: postgres://postgres@127.0.0.1:5433/local_testnet
Transaction stream is ready. Endpoint: http://0.0.0.0:50051/
Indexer API is ready. Endpoint: http://127.0.0.1:8090/
Faucet is ready. Endpoint: http://127.0.0.1:8081/

Applying post startup steps...

Setup is complete, you can now use the local testnet!
```

Once you see this final line, you know the local testnet is ready to use:
```
Setup is complete, you can now use the local testnet!
```

As you can see from the output, once the local network is running, you have access to the following services:
- [Node API](../nodes/aptos-api-spec.md): This is a REST API that runs directly on the node. It enables core write functionality such as transaction submission and a limited set of read functionality, such as reading account resources or Move module information.
- [Indexer API](../indexer/api/index.md): This is a GraphQL API that provides rich read access to indexed blockchain data. If you click on the URL for the Indexer API above, by default http://127.0.0.1:8090, it will open the Hasura Console. This is a web UI that helps you query the Indexer GraphQL API.
- [Faucet](../reference/glossary#faucet): You can use this to create accounts and mint APT on your local network.
- [Transaction Stream Service](../indexer/txn-stream/index.md): This is a grpc stream of transactions. This is relevant to you if you are developing a [custom processor](../indexer/custom-processors/index.md).
- Postgres: This is the database that the indexer processors write to. The Indexer API reads from this database.

## Using the local network

### Configuring your Aptos CLI

You can add a separate profile, as shown below:

```bash
aptos init --profile local --network local
```

and you will get an output like below. At the `Enter your private key...` command prompt press enter to generate a random new key.

```bash
Configuring for profile local
Using command line argument for rest URL http://localhost:8080/
Using command line argument for faucet URL http://localhost:8081/
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]
```

This will create and fund a new account, as shown below:

```bash
No key given, generating key...
Account 7100C5295ED4F9F39DCC28D309654E291845984518307D3E2FE00AEA5F8CACC1 doesn't exist, creating it and funding it with 10000 coins
Aptos is now set up for account 7100C5295ED4F9F39DCC28D309654E291845984518307D3E2FE00AEA5F8CACC1!  Run `aptos help` for more information about commands
{
  "Result": "Success"
}
```

From now on you should add `--profile local` to CLI commands to run them against the local network.

### Configuring the TypeScript SDK

In order to interact with the local network using the TypeScript SDK, use the local network URLs when building the client:
```typescript
import { Provider, Network } from "aptos";

const provider = new Provider(Network.LOCAL);
```

The provider is a single super client for both the node and indexer APIs.

## Resetting the local network

Sometimes while developing it is helpful to reset the local network back to its initial state:
- You made backwards incompatible changes to a Move module and you'd like to redeploy it without renaming it or using a new account.
- You are building a [custom indexer processor](../indexer/custom-processors/index.md) and would like to index using a fresh network.
- You want to clear all on chain state, e.g. accounts, objects, etc.

To start with a brand new local network, use the `--force-restart` flag:
```bash
aptos node run-local-testnet --force-restart
```

It will then prompt you if you really want to restart the chain, to ensure that you do not delete your work by accident.

```bash
Are you sure you want to delete the existing chain? [yes/no] >
```

If you do not want to be prompted, include `--assume-yes` as well:
```bash
aptos node run-local-testnet --force-restart --assume-yes
```

## FAQ

### Where can I get more information about the run-local-testnet command?

More CLI help can be found by running the command:

```bash
aptos node run-local-testnet --help
```

It will provide information about each of the flags you can use.


### I'm getting the error `address already in use`, what can I do?

If you're getting an error similar to this error:

```bash
'panicked at 'error binding to 0.0.0.0:9101: error creating server listener: Address already in use (os error 48)'
```

This means one of the ports needed by the local network is already in use by another process.

On Unix systems, you can run the following command to get the name and PID of the process using the port:

```bash
lsof -i :8080
```

You can then kill it like this:
```bash
kill $PID
```

### How do I change the ports certain services run on?

You can find flags to configure this for each service in the CLI help output:
```
aptos node run-local-testnet -h
```

The help output tells you which ports services use by default.

### How do I opt out of running certain services?

- Opt out of running a faucet with `--no-faucet`.
- Opt out of running a Transaction Stream Service with `--no-txn-stream`.


### How do I publish Move modules to the local testnet?

If you set up a profile called `local` above, you can run any command by adding the `--profile local` flag. In this case, we also use `local` as the named address in the `HelloBlockchain` example. The CLI will replace `local` with the account address for that profile.

```bash
aptos move publish --profile local --package-dir /opt/git/aptos-core/aptos-move/move-examples/hello_blockchain --named-addresses HelloBlockchain=local
```

### How do I see logs from the services?
In the output of the CLI you will see something like this:
```
Test dir: "/Users/dport/.aptos/testnet"
```

The logs from each of the services can be found in here. There are directories for the logs for each service. For processor logs, see the `tokio-runtime` directory.

### What if it says Docker is not available?
To run an Indexer API using `--with-indexer-api` you need to have Docker on your system.

You might be seeing an error that looks like this:
```
Unexpected error: Failed to apply pre run steps for Postgres: Docker is not available, confirm it is installed and running. On Linux you may need to use sudo
```

Make sure you have Docker 24+:
```bash
$ docker --version
Docker version 24.0.6, build ed223bc
```

Make sure the Docker daemon is running. If you see this error it means it is not running:
```bash
$ docker info
...
ERROR: Cannot connect to the Docker daemon at unix:///Users/dport/.docker/run/docker.sock. Is the docker daemon running?
```

Make sure the socket for connecting to Docker is present on your machine in the default location. For example on Unix systems this file should exist:
```
/var/run/docker.sock
```

If you're on Mac or Windows, we recommend you use Docker Desktop rather than installing Docker via a package manager (e.g. Homebrew or Choco).

### How do I use the Postgres on my host machine?
By default when using `--with-indexer-api` the CLI will run a Postgres instance in Docker. If you have Postgres running on your host machine and would like to use that instead, you can do so with the `--use-host-postgres` flag. There are also flags for specifying how it should connect to the host Postgres. Here is an example invocation:
```bash
aptos node run-local-testnet --with-indexer-api --use-host-postgres --postgres-user $USER
```

### How do I wait for the local network to come up programmatically?
When running the CLI interactively, you can see if the network is alive by waiting for this message:
```
Setup is complete, you can now use the local testnet!
```

If you writing a script and would like to wait for the local network to come up, you can make a GET request to `http://127.0.0.1:8070`. At first this will return [503](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/503). When it returns [200](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/200) it means all the services are ready.

You can inspect the response to see which services are ready.

<details>
<summary>Example using curl</summary>
<p>

```json
$ curl http://127.0.0.1:8070 | jq .
{
  "ready": [
    {
      "Http": [
        "http://127.0.0.1:43236/",
        "processor_default_processor"
      ]
    },
    {
      "Http": [
        "http://127.0.0.1:43240/",
        "processor_token_processor"
      ]
    },
    {
      "Http": [
        "http://127.0.0.1:43242/",
        "processor_user_transaction_processor"
      ]
    },
    {
      "Postgres": "postgres://postgres@127.0.0.1:5433/local_testnet"
    },
    {
      "Http": [
        "http://127.0.0.1:8081/",
        "Faucet"
      ]
    },
    {
      "IndexerApiMetadata": "http://127.0.0.1:8090/"
    },
    {
      "Http": [
        "http://127.0.0.1:8090/",
        "Indexer API"
      ]
    },
    {
      "NodeApi": "http://0.0.0.0:8080/"
    },
    {
      "Http": [
        "http://127.0.0.1:43239/",
        "processor_stake_processor"
      ]
    },
    {
      "DataServiceGrpc": "http://0.0.0.0:50051/"
    },
    {
      "Http": [
        "http://127.0.0.1:43235/",
        "processor_coin_processor"
      ]
    },
    {
      "Http": [
        "http://127.0.0.1:43237/",
        "processor_events_processor"
      ]
    },
    {
      "Http": [
        "http://127.0.0.1:43234/",
        "processor_account_transactions_processor"
      ]
    },
    {
      "Http": [
        "http://127.0.0.1:43241/",
        "processor_token_v2_processor"
      ]
    },
    {
      "Http": [
        "http://127.0.0.1:43238/",
        "processor_fungible_asset_processor"
      ]
    }
  ],
  "not_ready": []
}
```

</p>
</details>

### How do I learn more about the Aptos CLI?
If you are new to the Aptos CLI see this comprehensive [Aptos CLI user documentation](../tools/aptos-cli/use-cli/use-aptos-cli.md).
