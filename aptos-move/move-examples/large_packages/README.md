# Aptos Large Packages Framework

This module provides a framework for uploading large packages to the Aptos network, under standard
accounts or objects.
To publish using this API, you must divide your metadata and modules across multiple calls
into `large_packages::stage_code_chunk`.
In each pass, the caller pushes more code by calling `stage_code_chunk`.
In the final call, the caller can optionally set `publish_to_account`, `publish_to_object`,
or `upgrade_object_code`.
If any of these options are set, the package will be published, saving an extra transaction and
additional storage costs.

The above logic is currently implemented in the Python
SDK: [`aptos-python-sdk`](https://github.com/aptos-labs/aptos-python-sdk/blob/main/aptos_sdk/package_publisher.py).
Aptos CLI supports this as well:

- `aptos move publish [OPTIONS] --chunked-publish`
- `aptos move create-object-and-publish-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`
- `aptos move upgrade-object-package [OPTIONS] --address-name <ADDRESS_NAME> --chunked-publish`

# Usage

1. **Stage Code Chunks**:
    - Call `stage_code_chunk` with the appropriate metadata and code chunks.
    - Ensure that `code_indices` are provided from 0 to `last_module_idx`, without any
      gaps.

   ```move
   stage_code_chunk(
       &signer,
       metadata_chunk,
       code_indices,
       code_chunks,
       false,  // publish_to_account
       false,  // publish_to_object
       false,  // upgrade_object_code
       None,   // code_object
   );
   ```

2. **Publish or Upgrade**:
    - In the final call to `stage_code_chunk`, set one of the publishing flags to `true` to publish
      or upgrade the package:

   ```move
   stage_code_chunk(
       &signer,
       metadata_chunk,
       code_indices,
       code_chunks,
       true,  // publish_to_account
       false, // publish_to_object
       false, // upgrade_object_code
       None,  // code_object
   );
   ```

    - For object code upgrades, provide the `code_object`:

   ```move
   stage_code_chunk(
       &signer,
       metadata_chunk,
       code_indices,
       code_chunks,
       false,  // publish_to_account
       false,   // publish_to_object
       true,  // upgrade_object_code
       Some(code_object), // code_object
   );
   ```

# Notes

* Make sure LargePackages is deployed to your network of choice, you can currently find it both on
  mainnet and testnet at `0x1ee85dbf6ba5232729932110df479da160988f52276533a0c45b2924d10136d1`
* Ensure that `code_indices` have no gaps. For example, if code_indices are
  provided as [0, 1, 3] (skipping index 2), the inline function `assemble_module_code` will abort
  since `StagingArea.last_module_idx` is set as the max value of the provided index
  from `code_indices`, and `assemble_module_code` will lookup the `StagingArea.code` SmartTable from
  0 to `StagingArea.last_module_idx` in turn.
* This framework is designed to optimize large package uploads by reducing the number of
  transactions and storage costs.
