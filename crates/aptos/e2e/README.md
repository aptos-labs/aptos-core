# CLI test suite
This directory contains Python code to help with running the CLI test suite.

## Requirements
We use [Poetry](https://python-poetry.org/docs/#installation) for packaging and dependency management:

```
curl -sSL https://install.python-poetry.org | python3 -
```

Once you have Poetry, you can install the dependencies for the testing framework like this:
```
poetry config virtualenvs.in-project true  # This helps with IDE integration
poetry install
```

To learn how to use the CLI testing framework, run this:
```
poetry run python main.py -h
```

For example, using the CLI from an image:
```
poetry run python main.py --base-network mainnet --test-cli-tag nightly
```

Using the CLI from a local path:
```
poetry run python main.py -d --base-network mainnet --test-cli-path ~/aptos-core/target/debug/aptos
```

## Debugging
If you are get an error message similar to this:

### CPU architecture
```
docker: no matching manifest for linux/arm64/v8 in the manifest list entries.
```

Try running the poetry command with this env var:
```
DOCKER_DEFAULT_PLATFORM=linux/amd64 poetry run python main.py --base-network testnet --test-cli-path ~/aptos-core/target/debug/aptos
```
This makes the docker commands use the x86_64 images since we don't publish images for ARM.

### CLI config type
If you see an error like this:
```
Traceback (most recent call last):
  File "/Users/dport/a/core/crates/aptos/e2e/main.py", line 194, in <module>
    if main():
  File "/Users/dport/a/core/crates/aptos/e2e/main.py", line 156, in main
    run_helper.prepare()
  File "/Users/dport/a/core/crates/aptos/e2e/test_helpers.py", line 155, in prepare
    self.prepare_cli()
  File "/Users/dport/a/core/crates/aptos/e2e/test_helpers.py", line 189, in prepare_cli
    raise RuntimeError(
RuntimeError: When using --test-cli-path you must use workspace configuration, try running `aptos config set-global-config --config-type workspace`
```

It is because you are using the `--test-cli-path` flag but have configured the CLI to use the `global` config type. This is not currently compatible, you must switch the config type to workspace prior to running the E2E tests:
```
aptos config set-global-config --config-type workspace
```

## Writing new test cases
To write a new test case, follow these steps:
1. (Optional) Make a new file in [cases/](cases/) if none of the existing files seem appropriate.
1. Write a new function following these guidelines:
    1. Follow the naming scheme `test_*`.
    1. Decorate the function with the `test_case` decorator.
    1. If you want to assert something, do so by raising an exception (TestError has been provided for this purpose, but any old exception does the trick).
    1. Use the `RunHelper` to invoke CLI commands. Follow the example of other test cases.
1. Register the test in the `run_tests` function in [main.py](main.py). Note that the order matters here, later tests are allowed (and encouraged) to depend on the results of earlier tests. This way we can test truly end-to-end, beyond the span of a single invocation.

## Formatting:
```
poetry run isort .
poetry run black .
```
