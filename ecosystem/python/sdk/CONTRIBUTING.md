# Contributing Guide
## Publishing
To publish the SDK, follow these steps.

First, make sure you have updated the changelog and bumped the SDK version if necessary.

Configure Poetry with the PyPi credentials:

```
poetry config pypi-token.pypi <token>
```

You can get the token from our credential management system, search for PyPi.

Build and publish:
```
poetry publish --build
```
