# Bonding Curve Launchpad

## Overview
Bonding Curve Launchpad (BCL) - A fungible asset launchpad that doubles as a controlled DEX to create fairer token launches.

After creation of a new FA, it can, initially, only be traded on the launchpad. Dispatchable FA features are used to support the global freezing of tokens from external transfers. The creator of the FA can not premint to gain advantages against latter participants. Additionally, the usage of virtual liquidity prevents the typical overwhelming early adopter's advantage.

Once the APT threshold is met within the liquidity pair, the reserves are moved onto a public DEX, referred to as graduation. From there, the global freeze is disabled, allowing all participants to freely use their tokens.

### Key terms
**Graduation** - The process of moving the close-looped FA's liquidity reserves from the `bonding_curve_launchpad` to a public DEX, while enabling any and all transfers from FA owners. Public trading is then available, removing the need to consult the `bonding_curve_launchpad`'s held `transfer_ref` for the given FA.

**Virtual Liquidity** - To prevent early adopter's advantage, a pre-defined amount of virtual liquidity is assumed to exist in all APT reserves for liquidity pairs. Since the FA and APT reserves will be closer together in value, an early transfer won't be as dramatic for the number of tokens received. 


## This resource contains:
* (Dispatchable) Fungible Assets.
* Reusing stored signer vars (w/ [objects](https://velor.dev/move/move-on-velor/objects/)).
* External third party dependencies.
* E2E testing (w/ object deployments, APT creation).
* Using `rev` to specify feature branches ([Dispatchable FA](https://github.com/velor-chain/velor-core/commit/bbf569abd260d94bc30fe96da297d2aecb193644)).
* and more.

## In-depth info
### Description
The goal of a bonding curve launchpad is to create a more direct and open environment for FA launches. What does this mean?
* Less early adopter's advantage
* No pre-mints
* No allocation to private investors

Due to the technical nature of the blockchain, it can be hard to determine whether an FA is impacted by 
the above attributes. 
Although these are not inherently negative, many users have admitted to decreased interest 
in new FAs that include them.
This has led to an increase interest on improving "fairness" of FA launches, and one way to approach this is through 
a smart contract solution. 

`bonding_curve_launchpad` is one instance of a smart contract-based effort that accomplishes this.

### Dispatchable withdraw function
When using the Dispatchable FA standard, one can define a custom `withdraw` function 
(along with others, like deposit and derived_balance). This logic is executed every time the 
default `withdraw` is called, like during an FA transfer.

`bonding_curve_launchpad` takes advantage of this by conditionally checking if a liquidity 
pair state variable is valid. More specifically, the related FA's state variable `is_frozen` must 
be set to `false`, otherwise all withdraws will fail.

This prevents any transfers from occurring that happen outside the explicit usage of `transfer_ref`. Participants will 
be forced to only use their FAs within the context of `bonding_curve_launchpad`, until the respective FA's `is_frozen` 
is toggled to `true` during **graduation**.

### Trading function
Constant product formula is used within `liquidity_pairs` for calculating `get_amount_out`, similar to 
[UniswapV2 on Ethereum](https://docs.uniswap.org/contracts/v2/concepts/protocol-overview/how-uniswap-works). 
Although chosen for familiarity and simplicity's sake, in a production environment, one may look towards other 
trading functions.

One style could encompass sublinear functions to reward early adopters more heavily.


## Related files
### `bonding_curve_launchpad.move`
* Contains public methods (both entry and normal), for the end user.
* Creates and holds references (transfer) to the launched Fungible Asset through the FA's respective Object.
* Dispatches to `liquidity_pairs` to create a new pair between the launched FA and APT.
* Facilitates swaps between any enabled FA and APT.
### `liquidity_pairs.move`
* Creates and holds references to each liquidity pair, through a named objects.
* Contains business logic for performing swaps on each pair.
* Performs graduation ceremony to disable liquidity pair, and move all reserves to an external, third party DEX.
### `test_bonding_curve_launchpad.move`
* E2E tests.
### `move-examples/swap`
* Fungible Asset DEX example from the `velor-move` examples.


### Lifecycle of launched FA
#### Launching the FA and liquidity pair
1. User initiates FA creation on the `bonding_curve_launchpad` through `create_fa_pair(...)`.
   1. `bonding_curve_launchpad` creates a new dispatchable FA and assigns the `"FA Generator" Object` as the creator, by retrieving it's `signer` using a stored `extend_ref`. Importantly, the user initiating the FA does not have any permissions (mint, transfer, burn), preventing their ability to pre-mint. FA capabilities (`transfer_ref`) are stored within the FA's respective Object, for future transfers.
   2. The dispatchable `withdraw` function is defined to disallow all transfers between FA owners, until **graduation** is met. It does this through a state value associated with the FA's soon-to-be generated liquidity pair, called `is_frozen`, which is initially set to `true`. This prevents the FA from being traded on external DEXs or through external means. Transfers are only available through the limited `transfer_ref` stored on `bonding_curve_launchpad`, and restricted to swaps on the `liquidity_pairs`'s liquidity pair. 
   3. A pre-defined number of the FA is minted, and kept temporarily on `bonding_curve_launchpad`.
2. `bonding_curve_launchpad` creates a liquidity pair on the `liquidity_pairs` module, which follows the constant product formula.
   1. Liquidity pair is represented and stored as an Object. This allows for the reserves to be held directly on the object, rather than the `bonding_curve_launchpad` account.
   2. The entirety of minted FA + a pre-defined number of **virtual liquidity** for APT is deposited into the liquidity pair object. 
   3. Trading against the liquidity pair is enabled, but restricted to `public entry` functions found in `bonding_curve_launchpad`.
3. Optionally, the creator can immediately initiate a swap from APT to the FA.
#### Trading against the FA's associated liquidity pair
1. External users can swap APT to FA, or vice versa, through `public entry` methods available on `bonding_curve_launchpad`. 
   1. Although the normal `transfer` functionality of the FA is disabled by the custom dispatchable `withdraw` function, `bonding_curve_launchpad` can assist with swaps using it's stored `transfer_ref` from the FA's Object. `transfer_ref` is not impeded by the custom dispatchable function.
   2. The logic for calculating the `amountOut` of a swap is based on the constant product formula for reader familiarity. In a production scenario, a sub-linear trading function can assist in incentivizing general early adoption.
#### Graduating from `liquidity_pairs` to a public FA DEX
1. After each swap from APT to FA, when the APT reserves are increasing, a threshold is checked for whether the liquidity pair can **graduate** or not. The threshold is a pre-defined minimum amount of APT that must exist in the reserves. Once this threshold is met during a swap, **graduation** begins.
   1. The associated liquidity pair on the `liquidity_pairs` module is disabled by toggling `is_enabled`, preventing any more swaps against the pair. Additionally, the liquidity pair's `is_frozen` is disabled to allow owners to transfer freely. 
   2. The reserves from the liquidity pair, both APT and FA, are moved to an external, public third-party DEX as a new liquidity pair. In this case, the `velor-move` FA DEX example, called swap. 
   3. To prevent any wrongdoing from the `bonding_curve_launchpad` owner, any liquidity tokens received during the creation of the new liquidity pair on the third-party DEX will be sent to a dead address. Otherwise, the tokens could be used to drain the liquidity pair, at any time.



## How to test:
```console
velor move test --dev
```

## Example testnet deployments
[Bonding Curve Launchpad](https://explorer.velorlabs.com/account/0x0bb954c7dda5fa777cb34d2e35f593ddc4749f1ab260017ee75d1d216a551841/transactions?network=testnet)

[Swap DEX](https://explorer.velorlabs.com/account/0xe26bbe169db47aaa32349d253891af42134e1f6b64fef63f60105ec9ab6b240f/transactions?network=testnet?)

[Swap Deployer](https://explorer.velorlabs.com/account/0x4d51c99abff19bfb5ca3065f1e71dfc066c38e334def24dbac2b2a38bee8b946?network=testnet)


## How to deploy:
0. **Note:** Since the `swap` module we're relying on as a third party DEX isn't on-chain, you'll need to first:
    * Deploy the `swap` module on-chain. Or, if you're on the testnet, you can use the [already-deployed `swap` smart contract](https://explorer.velorlabs.com/account/0xe26bbe169db47aaa32349d253891af42134e1f6b64fef63f60105ec9ab6b240f/transactions?network=testnet?).
    * Rely on a different DEX for the token graduation.

From there, you can follow the [object code deployment](https://preview.velor.dev/en/build/smart-contracts/learn-move/advanced-guides/object-code-deployment) steps to deploy and set up the smart contract.

### Testnet deployment
Deploy the `bonding_curve_launchpad` to the testnet referencing the already-deployed `swap` smart contract:
```console
velor move publish --profile testnet_bonding_curve_launchpad \
--named-addresses bonding_curve_launchpad={REPLACE_WITH_YOUR_ACCOUNT},swap=0xe26bbe169db47aaa32349d253891af42134e1f6b64fef63f60105ec9ab6b240f,deployer=0x4d51c99abff19bfb5ca3065f1e71dfc066c38e334def24dbac2b2a38bee8b946
```