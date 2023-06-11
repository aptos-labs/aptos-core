# Modular Whitelist

This is a modular whitelist smart contract intended to gate an account's access to do "Something" based on time, price, and the # of times they've already done "Something".

The contract leverages objects by giving a whitelist object to the creator and using it to manage the whitelisting logistics.

The intention is to offer a modular whitelist that can be used by a separate smart contract without having to write too much boilerplate code.

## Setting up the whitelist
```
whitelist::init_tiers(owner);

whitelist::upsert_tier_config(
	owner,
	string::utf8(b"public"),
	true, // open_to_public, users don't need to be registered in the list
	PUBLIC_PRICE,
	PUBLIC_START_TIME,
	PUBLIC_END_TIME,
	PUBLIC_PER_USER_LIMIT,
);

whitelist::upsert_tier_config(
	owner,
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
public entry fun mint(...) {
	// ... other minting logic

	// update the account's info in the corresponding tier
   whitelist::deduct_one_from_tier(owner, receiver, tier_name);

	// ... other minting logic
}
```

## Clean up
```
// the owner of the whitelist object can destroy it
destroy(owner);
```

Whitelists are most commonly used with minting contracts, so we show an example that involves a very simple minting smart contract.

Unit tests in whitelist.move and mint.move can be run by specifying `@whitelist_example` as a named address.

```
aptos move test --named-addresses whitelist_example=default
```