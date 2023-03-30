# CLI test suite
This directory contains Python code to help with running the CLI test suite.

## Requirements
We use [Poetry](https://python-poetry.org/docs/#installation) for packaging and dependency management:

```
curl -sSL https://install.python-poetry.org | python3 -
```

Once you have Poetry, you can install the dependencies for the testing framework like this:
```
poetry install
```

To learn how to use the CLI testing framework, run this:
```
poetry run python main.py -h
```

For example:
```
poetry run python main.py --base-network mainnet --test-cli-tag mainnet
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
