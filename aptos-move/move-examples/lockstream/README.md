# Lockstream demo

Unveiled at Aptos Hack Singapore 2023

## Overview

The example demo compiles an Aptos CLI from source and uses it to run a local testnet.

Local vanity address keys for `Ace`, `Bee`, `Cad`, and `Dee` (generated using [Econia Labs' `optivanity` too](https://github.com/econia-labs/optivanity)) are used to fund the accounts with `APT` during the image build phase.

The coin types `DeeCoin` and `USDC` are published under `Dee`'s account, which creates the Lockstream and seeds it with `10000` `DeeCoin`.

Ace, Bee, and Cad lock `USDC` as follows:

|                   | Ace | Bee | Cad |
| ----------------- | --- | --- | --- |
| Lock txn 1        | 100 |     |     |
| Lock txn 2        |     | 200 |     |
| Lock txn 3        |     |     | 300 |
| Lock txn 4        | 400 |     |     |
| Total USDC locked | 500 | 200 | 300 |

Hence each locker is entitled to their pro rata share of `DeeCoin`:

| Ace  | Bee  | Cad  |
| ---- | ---- | ---- |
| 5000 | 2000 | 3000 |

Period times:

| Period                | Time in seconds |
| --------------------- | --------------- |
| Locking period        | 20              |
| Streaming period      | 60              |
| Claiming grace period | 30              |
| Premier sweep period  | 30              |

`DeeCoin` claim amounts

| Time (s) since stream period start | Ace  | Bee  | Cad  |
| ---------------------------------- | ---- | ---- | ---- |
| 15                                 | 1250 |      |      |
| 30                                 |      | 1000 |      |
| 45                                 |      |      | 2250 |
| 60                                 | 3750 |      |      |
| 70                                 |      | 1000 |      |

(For simplicity, sweep operations are not demonstrated during the demo.)

## Running the example

Running the Docker compose your first time:

> This might take a while the first time, since it compiles several Aptos binaries from source.

```sh
# From inside aptos-move/move-examples/lockstream
docker compose --file docker-compose.yaml up
```

Then press Ctrl+C to shut down the local testnet.

To run the example again with new containers (fresh chain state) each time:

```sh
# From inside aptos-move/move-examples/lockstream
docker compose --file docker-compose.yaml up --force-recreate
```

(Pro tip) If you want to experiment with the example, use the following to clear all containers and images, composing directly from the Docker build cache:

```sh
docker rm -vf $(docker ps -aq)
docker rmi -f $(docker images -aq)
docker compose --file docker-compose.yaml up
```
