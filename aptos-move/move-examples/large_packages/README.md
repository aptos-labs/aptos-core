This package provides an experimental service for uploading very large modules to the Aptos network. To publish using this API, you must divide your metadata and modules across multiple calls into `large_packages::stage_code`. Specifically:

* Make sure LargePackages is deployed to your network of choice, you can currently find it on testnet at `0xd20f305e3090a24c00524604dc2a42925a75c67aa6020d33033d516cf0878c4a`
* Compile your package
* Chunk up the metadata and modules and call `large_packages::stage_code`
* In your last call to `large_packages::stage_code` set `publish` to `true`

The above logic is currently implemented in the Python SDK: `aptos-core/ecosystem/python/sdk/aptos_sdk/package_publisher.py`

For validation purposes, this contains a package, `large_package_example` that exceeds the requirements for publishing in a single transaction.

This framework has some limitations:
* There is no consistency checking until the publishing attempt
* Module code is not split across chunks, so if a single module is too big, it won't work
