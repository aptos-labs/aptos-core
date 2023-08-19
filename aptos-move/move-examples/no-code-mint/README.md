# No Code Mint Machine

This example demonstrates how to create a programmatic minting contract that doesn't require the creator to sign off on individual transactions requesting to mint.

There are multiple modules here, their individual purposes are described below.


## Deploying and upgrading the contract (package_manager.move)

You can deploy the contract yourself or use an existing version of it somewhere else. For granular control over your own collection, it is suggested you
deploy the contract yourself.

### Deploying the module

You must deploy the contract through the package_manager, because the deploying account is assumed to be a resource account. The contract data is stored at the contract's address and accessed through the package_manager module.

To simplify the following process, we assume you've exported your profile name as an environment variable:

```shell
export NO_CODE_MINT_DEPLOYER="default"
```

Here's how you would deploy and upgrade with the Aptos CLI:

```shell
cd ~/aptos-core/aptos-move/move-examples/no-code-mint &&
aptos move create-resource-account-and-publish-package --named-addresses deployer=$NO_CODE_MINT_DEPLOYER --address-name no_code_mint --profile $NO_CODE_MINT_DEPLOYER --seed 0
```

The resulting resource account is an upgradeable module with the contract data stored at its address. Store this resource account's address in an environment variable as well:

```shell
export NO_CODE_MINT_RESOURCE_ADDRESS="0xYOUR_RESOURCE_ACCOUNT_ADDRESS"
```

### Upgrading the module

If you want to upgrade the module after having deployed it, you must use the package_manager's `publish_package` function, where you pass in the serialized metadata and code.

You can get this by running the following command:

```shell
aptos move build-publish-payload --named-addresses deployer=$NO_CODE_MINT_DEPLOYER,no_code_mint=$NO_CODE_MINT_RESOURCE_ADDRESS --profile $NO_CODE_MINT_DEPLOYER --json-output-file publish_data.json
```

You then need to update the json file so that the entry function is the fully qualified `$NO_CODE_MINT_RESOURCE_ADDRESS::package_manager::publish_package` function tag.

If you have jq installed, you can do it with this command:

```shell
jq --arg newFunctionID "$NO_CODE_MINT_RESOURCE_ADDRESS::package_manager::publish_package" '.function_id |= $newFunctionID' publish_data.json > temp.json && mv temp.json publish_data.json
```

Once you've changed the function_id in `publish_data.json`, run that code as a json file to update the package:

```shell
aptos move run --json-file publish_data.json --profile $NO_CODE_MINT_DEPLOYER
```

## The Mint Machine (mint_machine.move, allowlist.move)

This is the main module, used to create the mint machine and called by the end user to mint from once the mint machine is initialized.

It utilizes the allowlist module to gate access to minting by allowlists, specifically whether or not the address requesting to mint is in a valid tier list.
That is, they have enough AptosCoin to pay for the mint, they aren't too early/late, and their address exists in the specified allowlist tier or the tier is public.

The process for initializing, configuring, and running a mint machine is as follows:

### Initialization
1. Initialize the mint machine with the `initialize_mint_machine` function as the `admin`.
    - The `admin` here is the user account that owns the mint machine object. It manages the mint machine object's resources
    - The collection is created here by the MintConfiguration object, also referred to as the creator object
    - Once created, it is undeletable due to being a named object.
    - The `initialize_mint_machine` function is where you specify most of the mutability configurations.
    - You also specify the token name base here. Tokens will be minted pseudo-randomly.
    - If your token_name_base is `Token #` and you have a max supply of 1000, then the tokens might appear like this:
        - `Token #573`, `Token #34`, `Token #999`, `Token #187`
    - If you want to use custom names, you can remove this functionality and store the token name in the TokenMetadata resource and use that
### Configuration
2. Create an allowlist with the `upsert_tier` function (as the admin)
    - For each tier, you must specify the following:
        - Tier name
        - Open to the public, that is, is it an allowlist (not public) or merely a registry that tracks # of mints (public)
        - Price
        - Start time
        - End time
        - Per user limit (# of mints)
    - If you do not create a valid allowlist, you will not be able to start the mint machine.
        - You must either have a public allowlist or a gated allowlist with at least one address in it.
    - You can create multiple allowlists.
    - If a user exists under multiple allowlists, the allowlist contract will mint from the earliest, cheapest one
3. Add `max_supply` token metadata to the MintConfiguration metadata table by calling the `add_tokens` function (as the admin)
    - The token metadata consists of:
        - The token uri
        - The token description
        - The property map keys, values, and types
            - Upon addition to the metadata table, if you run the function with the `safe` flag, each property map supplied will be verified
                as correctly serialized. This is to avoid errors that would only otherwise occur upon mint
    - This metadata is applied to the token object created upon mint
    - When adding metadata:
        - Each token metadata is stored in the metadata_table, indexed with token uris as its keys
        - The token uri is pushed onto a vector of token_uris, which is pseudorandomly popped from later when a user mints.
    - The metadata table exists to ensure the uniqueness of token uris
    - The metadata vector exists because it offers the `swap_remove` function, used in the prng process. Tables do not have this functionality
    - Note again that the names are random in the sense that their current # in the existing vector determines what's appended to the end of the base_token_name
### Enabling the mint machine
4. Enable minting with `enable_minting` (as the admin)
    - This function will fail if you have not configured the MintConfiguration properly, namely, the following must be true:
        - The mint machine has a allowlist with a valid tier (either public or at least 1 address allowlisted)
        - The metadata table length equals the max supply of the collection

The minting machine has now begun, and if there is a valid tier in the allowlist to mint from, an eligible user may mint.

### Users mint
When the user requests to mint, they pay the mint price and the token object is created with its corresponding metadata. It is then transferred to the minter.
This repeats until the collection has reached its max supply or the last tier's end time has been reached.

Note: Steps 2 and 3 are interchangeable, they can be done in either order.

If you'd like to see the Move representation of the e2e flow from initialization to the first mint, see the `mint_machine::test_happy_path` unit test.

### Clean up

You can destroy the whitelist object oncoe minting is complete by calling the `destroy_allowlist` function as the admin.
