# Aptos Large Packages Framework

This module provides a framework for uploading large packages to the Aptos network, under standard
accounts or objects.
To publish using this API, you must divide your metadata and modules across multiple calls
into `large_packages::stage_code_chunk`.
In each pass, the caller pushes more code by calling `stage_code_chunk`.
In the final call, the caller can use `stage_code_chunk_and_publish_to_account`, `stage_code_chunk_and_publish_to_object`, or
`stage_code_chunk_and_upgrade_object_code` to upload the final data chunk and publish or upgrade the package on-chain.

The above logic is currently implemented in the Python
SDK: [`aptos-python-sdk`](https://github.com/aptos-labs/aptos-python-sdk/blob/main/aptos_sdk/package_publisher.py).

Aptos CLI supports this as well with `--chunked-publish` flag:
- `aptos move publish [OPTIONS] --chunked-publish`
- `aptos move create-object-and-publish-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`
- `aptos move upgrade-object-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`

# Usage

1. **Stage Code Chunks**:
    - Call `stage_code_chunk` with the appropriate metadata and code chunks.
    - Ensure that `code_indices` are provided from `0` to `last_module_idx`, without any
      gaps.


2. **Publish or Upgrade**:
    - In order to upload the last data chunk and publish the package, call `stage_code_chunk_and_publish_to_account` or `stage_code_chunk_and_publish_to_object`.

    - For object code upgrades, call `stage_code_chunk_and_upgrade_object_code` with the argument `code_object` provided.

3. **Cleanup**:
    - In order to remove `StagingArea` resource from an account, call `cleanup_staging_area`.

# Notes

* Make sure LargePackages is deployed to your network of choice, you can currently find it both on
  mainnet and testnet at `0xa29df848eebfe5d981f708c2a5b06d31af2be53bbd8ddc94c8523f4b903f7adb`
* Ensure that `code_indices` have no gaps. For example, if code_indices are
  provided as [0, 1, 3] (skipping index 2), the inline function `assemble_module_code` will abort
  since `StagingArea.last_module_idx` is set as the max value of the provided index
  from `code_indices`, and `assemble_module_code` will lookup the `StagingArea.code` SmartTable from
  0 to `StagingArea.last_module_idx` in turn.
