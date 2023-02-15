---
title: "Airdrop Aptos Tokens"
id: "airdrop-aptos-tokens"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Airdrop Aptos Tokens

We support [different ways](./aptos-token.md#token-transfer) of transferring tokens in the Aptos token standard. In this example, we use the two-step process to airdrop to a list of addresses since this method doesn't require the receiver to opt-in to direct transfer.

The two-steps are:

1. The app offering the token to an account.
2. The receiver claiming this token.

After minting a token or using existing tokens from TokenStore, we can directly offer the token to a receiver address from a whitelist.

Starting with the [NFT mint example](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/move-examples/mint_nft/2-Using-Resource-Account/sources/create_nft_with_resource_account.move), we can change the `mint` function to an `airdrop` function as below.

This function directly offers the token to a list of addresses:

```rust

public entry fun airdrop(whitelist: vector<address>) acquires ModuleData {
    let module_data = borrow_global_mut<ModuleData>(@mint_nft);
    
    let count = vector::length(&whitelists);
    let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
    let token_id = token::mint_token(&resource_signer, module_data.token_data_id, count);
    
    let i: u64 = 0;
    while(i < count) {
        let receiver = vector::pop_back(&mut whitelist);
        token_transfers::offer(&resource_signer, receiver, token_id, 1);
        i = i + 1;
    };
}
```

After offering the token, the wallet receiver (eg: [Petra](https://petra.app/)) would see the offer as shown below:
![petra_screenshot.png](../../../static/img/token-airdrop.png)

This completes the Aptos token airdrop example.