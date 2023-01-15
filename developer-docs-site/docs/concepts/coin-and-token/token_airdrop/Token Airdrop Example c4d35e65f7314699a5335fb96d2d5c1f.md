# Token Airdrop Example

We support different ways of transferring token in our token standard [[link](https://aptos.dev/concepts/coin-and-token/aptos-token#token-transfer)]

With the 2 step token transfer, airdropping NFTs to the a set of address can be very straight-ward. 

After minting a token or using existing token from TokenStore, we can directly offer the token to an receiver address from a whitelisted lists. Compared to the NFT mint [example](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/move-examples/mint_nft/2-Using-Resource-Account/sources/create_nft_with_resource_account.move), we can change the `mint` function to `airdrop` function as below.  It can directly offer the token to a list of addresses 

```rust

public entry fun airdrop() acquires ModuleData {
    let module_data = borrow_global_mut<ModuleData>(@mint_nft);

    let count = big_vector::length(&whitelisted_addresses);
    let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
    let token_id = token::mint_token(&resource_signer, module_data.token_data_id, count);

    let i: u64 = 0;
    while (i < count){
        let receiver = big_vector::borrow(&whitelisted_addresses, i);
        **token_transfer::offer(resource_signer, receiver, token_id, 1);**
        i = i + 1;
    };
}
```

After offering the token, the wallet receiver (eg: Petra) would see the offer as shown below.

![Screenshot 2023-01-12 at 5.35.32 PM.png](Token%20Airdrop%20Example%20c4d35e65f7314699a5335fb96d2d5c1f/Screenshot_2023-01-12_at_5.35.32_PM.png)