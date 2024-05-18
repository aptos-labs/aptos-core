# Module Description

This Move module implements a simple rewards system, allowing an admin to manage and distribute rewards to users.

## Constants
- `ENOT_AUTHORISED: u64 = 1` - Caller is not authorized to perform the action.
- `ENO_REWARDS_TO_CLAIM: u64 = 2` - No rewards to claim.

## Structs
- `RewardStore`:
  - `admin: address`
  - `rewards: SmartTable<address, Coin<AptosCoin>>`

## Functions
- `init_module(rewards_signer: &signer)` - Initializes the module with the rewards store and sets the admin to the provided signer's address.
- `pending_rewards(user: address): u64 acquires RewardStore` - Returns the pending rewards for the caller. If no rewards are found, returns 0.
- `is_admin(admin: address): bool acquires RewardStore` - Checks if the provided address is the admin.
- `add_rewards(admin: &signer, recipients: vector<address>, amounts: vector<u64>) acquires RewardStore` - Allows the admin to add rewards for multiple recipients.
- `cancel_rewards(admin: &signer, recipients: vector<address>) acquires RewardStore` - Allows the admin to cancel rewards for specified recipients.
- `claim_reward(user: &signer) acquires RewardStore` - Allows users to claim their rewards. Errors out if there are no rewards to claim.
- `transfer_admin_role(admin: &signer, new_admin: address) acquires RewardStore` - Transfers the admin role to a new address.
- `init_for_test(admin: &signer)` - Initializes the reward store for testing purposes with the caller as the admin.

# Deployment
This rewards can be directly deployed to an account. It's upgradable by default but can be made immutable by adding upgrade_policy = "immutable" to the top section in the Move.toml file.
To deploy:
1. Create a profile for the rewards account. This is where the code will be deployed to and this account will also be the admin who can add/cancel rewards.
```bash
aptos move init --profile rewards
```
2. Run the following command in the simple directory:
```bash
aptos move publish --named-addresses rewards=rewards --profile rewards
```

# Testing
```bash
aptos move test --named-addresses rewards=0xcafe
```

# Testing Functionality
Replace 0x123 with the actual contract address (same as the rewards account's address). Replace the other addresses with the intended ones too. 

## Add rewards
1. Fund the rewards (admin) account with enough APT to cover the rewards.
2. Run the following command to add rewards. Only the admin account can run this.
```bash
aptos move run --profile rewards \
  --function-id 0x123::rewards::add_rewards \
  --args address:[0x234,0x235] u64:[1000,1000]
```

## Cancel rewards
To cancel the rewards for specific accounts, run the following command. Only the admin account can run this.
```bash
aptos move run --profile rewards \
  --function-id 0x123::rewards::cancel_rewards \
  --args address:[0x234,0x235]
```

## Claim reward
To claim rewards for the caller, run the following command. The caller must have rewards to claim.
```bash
aptos move run --profile user \
  --function-id 0x123::rewards::claim_reward
```

## Transfer Admin Role
To transfer the admin role to a new address, run the following command. Only the current admin can perform this action.

```bash
aptos move run --profile admin \
  --function-id 0x123::rewards::transfer_admin_role \
  --args address:0x456
```

## View Pending Rewards
To check the pending rewards for a specific user, run the following command. This can be called by anyone.

```bash
aptos move view --function-id 0x123::rewards::pending_rewards \
  --profile user \
  --args address:0x789
```

## View Admin Status
To check if a specific address is the admin, run the following command. This can be called by anyone.

```bash
aptos move view --function-id 0x123::rewards::is_admin \
  --profile user \
  --args address:0x789
```



