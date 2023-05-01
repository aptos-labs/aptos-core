# Faucet integration tests
This directory contains Python code to help with running the faucet integration tests. It takes care of spinning up a local testnet, moving the mint key where it is expected, checking that a Redis server is up, and running the integration tests.

## Requirements
We use [Poetry](https://python-poetry.org/docs/#installation) for packaging and dependency management:

```
curl -sSL https://install.python-poetry.org | python3 -
```

Once you have Poetry, you can install the dependencies for the testing framework like this:
```
poetry install
```

## Running
First, run a local Redis 6 server ([installation guide](https://redis.io/docs/getting-started/)).
```
redis-server
redis-cli flushall
```

To learn how to use the testing framework, run this:
```
poetry run python main.py -h
```

For example:
```
poetry run python main.py --base-network mainnet
```

## Formatting:
```
poetry run isort .
poetry run black .
```
