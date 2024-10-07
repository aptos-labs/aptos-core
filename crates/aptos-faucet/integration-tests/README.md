# Faucet integration tests
This directory contains Python code to help with running the faucet integration tests. It takes care of spinning up a localnet, moving the mint key where it is expected, checking that a Redis server is up, and running the integration tests.

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
poetry run python main.py --base-network testnet
```

## Debugging
If you are get an error message similar to this:
```
docker: no matching manifest for linux/arm64/v8 in the manifest list entries.
```

Try running the poetry command with this env var:
```
DOCKER_DEFAULT_PLATFORM=linux/amd64 poetry run python main.py --base-network testnet
```
This makes the docker commands use the x86_64 images since we don't publish images for ARM.

## Formatting:
```
poetry run isort .
poetry run black .
```

