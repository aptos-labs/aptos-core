# Modular Whitelist

This is a simple whitelist smart contract intended to gate an account's access to do "Something" based on time, price, and the # of times they've already done "Something".

The intention is to offer a modular whitelist that can be used by a separate smart contract without having to write too much boilerplate code.

## Setting up the whitelist
```
whitelist::init_tiers(resource_signer);

whitelist::upsert_tier_config(
	resource_signer,
	string::utf8(b"public"),
	true, // open_to_public, users don't need to be registered in the list
	PUBLIC_PRICE,
	PUBLIC_START_TIME,
	PUBLIC_END_TIME,
	PUBLIC_PER_USER_LIMIT,
);

whitelist::upsert_tier_config(
	resource_signer,
	string::utf8(b"whitelist"),
	false, // open_to_public, users need to be registered in the whitelist
	WHITELIST_PRICE,
	WHITELIST_START_TIME,
	WHITELIST_END_TIME,
	WHITELIST_PER_USER_LIMIT,
);
```

## Using the whitelist to count the # of interactions
```
public entry fun mint(receiver: &signer, tier_name: String) acquires MintConfiguration {
	// ... other minting logic

	// update the account's info in the corresponding tier
	whitelist::deduct_one_from_tier(receiver, resource_signer, tier_name);

	// ... other minting logic
}
```

Whitelists are most commonly used with minting contracts, so we show an example that involves a very simple minting smart contract, although it's not necessary, just shown for the sake of conveying the typical collection creator and subsequent user minting flow.

```