# Deployment
This describes the deployment flow where the contract is upgradable and controlled via an admin account.
1. Create a mainnet profile for mainnet. This account can be thrown away after the full deployment.
   aptos init --profile deployer
2. Deploy with the following command. The output should include the address of the deployed contract.
   aptos move create-object-and-publish-package --address-name rewards --profile deployer
3. Test and make sure the admin account can add rewards
4. Create a multisig account via RimoSafe UI: https://www.rimosafe.com/. The multisig can be created with Ledger accounts via Petra.
5. Transfer upgrade permission to the multisig with the contract address and multisig address replaced in the command below:
   aptos move run-function --function-id 0x1::object::transfer_raw --args address:contract_address address:multisig_address --profile deployer
6. Transfer admin to the multisig with
   aptos move run-function --function-id contract_address::rewards::transfer_admin --args address:multisig_address --profile deployer
7. Via RimoSafe UI, accept the admin transfer by proposing and executing a call to contract_address::rewards::accept_admin (no args necessary)

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