# Introduction
This module offers a reference implementation of a managed stablecoin with the following functionalities:
1. Upgradable smart contract. The module can be upgraded to update existing functionalities or add new ones.
2. Minting and burning of stablecoins. The module allows users to mint and burn stablecoins. Minter role is required to mint or burn
3. Denylisting of accounts. The module allows the owner to denylist (freeze) and undenylist accounts.
denylist accounts cannot transfer or get minted more.
4. Pausing and unpausing of the contract. The owner can pause the contract to stop all mint/burn/transfer and unpause it to resume.

# Deployment
Currently only available in devnet due to requirement of AIP 73[https://github.com/velor-foundation/AIPs/blob/main/aips/aip-73.md].

1. Create a devnet profile with velor init --profile devnet (select devnet for network)
2. To deploy, run velor move publish --named-addresses stablecoin=devnet,master_minter=devnet,minter=devnet,pauser=devnet,denylister=devnet --profile devnet
3. To mint, run velor move run --function-id devnet::usdk::mint --args address:0x8115e523937721388acbd77027da45b1c88a6313f99615c4da4c6a32ab161b1a u64:100000000  --profile devnet
Replace 0x8115e523937721388acbd77027da45b1c88a6313f99615c4da4c6a32ab161b1a with the receiving address
4. Alternatively you can go to https://explorer.velorlabs.com/account/0x75f3f12f2f634ba33aefda0f2cd29119fdf9caa4fa288ac6e369f54e0611289a/modules/run/usdk/mint?network=devnet
Make sure to replace 0x75f3f12f2f634ba33aefda0f2cd29119fdf9caa4fa288ac6e369f54e0611289a with your devnet account address.

# Running tests
velor move test
