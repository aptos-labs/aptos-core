---
title: "Python SDK"
slug: "python-sdk"
---

# Python SDK

Aptos provides a lightly maintained official Python SDK. It is available on [PyPi](https://pypi.org/project/aptos-sdk/) with the source code in the [Aptos-core GitHub repository](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk). Much of the functionality of the Python SDK mirrors the [Typescript SDK](/sdks/ts-sdk/index). The primary purpose of the Python SDK is to help Python developers to quickly become familiar with Aptos and as an accompaniment to Aptos tutorials.

## Installing Python SDK

The Python SDK can either be installed via `pip`, from source, or embedded:

### Install with pip

To install via `pip`:

```bash
pip3 install aptos-sdk
```

The `aptos-sdk` will be installed in the local site packages directory. For example, on macOS, you will find the `aptos-sdk` in the `~/Library/Python/3.8/lib/python/site-packages/aptos_sdk` directory.

### Install from the source

To install from source:

```bash
git clone https://github.com/aptos-labs/aptos-core
cd aptos-core/ecosystem/python/sdk
python3 setup.py install --user
```

### Install by embedding

To embed the Python SDK into your existing Python project:

```
cd /path/to/python/project
cp -r /path/to/aptos-core/ecosystem/python/sdk/aptos-sdk aptos-sdk
```

## Using Python SDK

See the [Developer Tutorials](/tutorials/index.md) for code examples showing how to use the Python SDK.