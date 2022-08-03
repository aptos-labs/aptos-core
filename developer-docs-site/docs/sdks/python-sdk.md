---
title: "Python SDK"
slug: "python-sdk"
---

# Aptos Python SDK

Aptos has an officially, lightly maintained Python SDK. It is available on [PyPi](https://pypi.org/project/aptos-sdk/) with the source code in the [Aptos-core github repository](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk). Much of the functionality mirrors our [Typescript SDK](transactions-with-ts-sdk). The primary purpose of this SDK is to help Python developers quickly acclimate to Aptos and as an accompaniment to our tutorials.

The Python SDK can either be installed via `pip`, from source, or embedded:

To install via `pip`:
```bash
pip3 install aptos-sdk
```

To install from source:
```bash
git clone https://github.com/aptos-labs/aptos-core
cd aptos-core/ecosystem/python/sdk
python3 setup.py install
```

To embed:
```
cd /path/to/python/project
cp -r /path/to/aptos-core/ecosystem/python/sdk/aptos-sdk aptos-sdk
```
