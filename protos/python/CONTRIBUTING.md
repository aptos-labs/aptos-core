# Velor Protos

## Regenerating code
Note: This is only for debugging. For normal uses, just use scripts/build_protos.sh in the parent directory.

Install deps:
```
poetry install
```

Generate code:
```
poetry run poe generate
```

## Publishing
To publish the package, follow these steps.

First, make sure you have updated the changelog and bumped the package version if necessary.

Configure Poetry with the PyPi credentials:
```
poetry config pypi-token.pypi <token>
```

You can get the token from our credential management system, search for PyPi.

Build and publish:
```
poetry publish --build
```
