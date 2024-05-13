# Bonding Curve Launchpad

## Overview
Bonding Curve Launchpad (BCL) - A fungible asset launchpad that doubles as a controlled DEX to create fairer token launches.

After creation of a new FA, it can, initially, only be traded on the launchpad. Dispatchable FA features are used to support the global freezing of tokens from external transfers. The creator of the FA can not premint to gain advantages against latter participants. Additionally, the usage of virtual liquidity prevents the typical overwhelming early adopter's advantage.

Once the APT threshold is met within the liquidity pair, the reserves are moved onto a public DEX, referred to as graduation. From there, the global freeze is disabled, allowing all participants to freely use their tokens.

## This resource contains:
* (Dispatchable) Fungible Assets.
* Stored signer vars (w/ [resource accounts](https://aptos.dev/tutorials/programmatic-upgradeable-module/#how-to-publish-modules-to-a-resource-account)).
* External third party dependencies.
* E2E testing (w/ resource accounts, APT creation).
* Using `rev` to specify feature branches ([Dispatchable FA](https://github.com/aptos-labs/aptos-core/commit/bbf569abd260d94bc30fe96da297d2aecb193644)).
* and more.

## How to test:
`aptos move test`

## How to deploy:
0. **Note:** Since the `swap` module we're relying on as a third party DEX isn't on-chain, you'll need to first:
    * Deploy the `swap` module on-chain.
    * Rely on a different DEX for the token graduation.


1. Initialize a profile, and make note of the address:
`aptos init`
```
Aptos CLI is now set up for account {PROFILE_ADDRESS} as profile default! ...
```

2. Derive the resource-account-address to be used:
```
aptos account derive-resource-account-address --address {PROFILE_ADDRESS} --seed {RANDOM_STRING_SEED}
```
```
{ "Result": {RESOURCE_ACCOUNT_ADDRESS} }
```

> Examples of `{RANDOM_STRING_SEED}` are: `random3`, `big_string_big_thoughts`, ... 

3. In `Move.toml`, replace `owner_addr` and `memecoin_creator_addr` with `{PROFILE_ADDRESS}`.

4. in `Move.toml`, replace `resource_account` with `0x{RESOURCE_ACCOUNT_ADDRESS}`.

5. Create and publish the package:
```
aptos move create-resource-account-and-publish-package --address-name {PROFILE_ADDRESS} --seed {RANDOM_STRING_SEED}
```