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

# Add rewards
1. Fund the rewards account with enough APT to cover the rewards.
2. Run the following command to add rewards. Only the admin account can run this.
```bash
aptos move run --profile rewards \
  --function-id 0x123::rewards::add_rewards \
  --args address:[0x234,0x235] u64:[1000,1000]
```
Replace 0x123 with the actual contract address (same as the rewards account's address)
3. To cancel the rewards for a specific account, run the following command. Only the admin account can run this.
```bash
aptos move run --profile rewards \
  --function-id 0x123::rewards::cancel_rewards \
  --args address:[0x234,0x235] u64:[1000,1000]
```
4. Users can claim rewards by calling the claim_rewards function via a custom built FE.