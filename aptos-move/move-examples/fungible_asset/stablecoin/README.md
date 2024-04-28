## Module Summary

This module implements several functionalities of the stablecoin design as described in the [Token Design Document](https://github.com/circlefin/stablecoin-evm/blob/master/doc/tokendesign.md).

### Implemented Functionalities
- **Initialization**: Initialize the module with the owner, master minter, maximum supply, name, symbol, and decimals.
- **Minting**: Mint new fiat tokens and deposit them to the `to` address.
- **Burning**: Burn fiat tokens from the minter's primary store.
- **Transfer**: Transfer fiat tokens from `from` address to `to` address.
- **Minter Management**:
  - Add a new minter with an allowance or update the minter's allowance (delegated minters).
  - Remove a minter from the minter's list.
- **Master Minter**: Update the master minter address.
- **View Functions**:
  - `get_metadata_object`
  - `is_blacklisted`
  - `is_minter`
  - `get_minter_allowance`
  - `balance_of_primary_store`
- **Test Functions**:
  - `test_initialization`
  - `test_mint`
  - `test_burn` (Note: more tests needed to expand coverage).

### Major Features Remaining
- Implementation of all V1.1, 2, and 2.1 features.
- **Pausing of Contract**: At fungible asset/store level and/or fiat token level.
- **Delegated Spending**: 
  - If a specific amount is required from Circle, consider modifying and further restricting access to transfer_ref.
  - Evaluate the benefits of allowing anyone to be a delegated spender.
- **Blacklisting**:
  - Implement frozen stores, creation of secondary stores using Object constructor ref.
  - Added security layer with blacklist vector.
- **Master Minter Module**:
  - Setup the correct governance model (owner-controller-worker model).
- **Rescuer Concept for Aptos**: Implement if required.
- **Role Management**: Use an Object for storing all roles.
- **Upgradeability**: Considerations and potential changes.

### Implementation Notes
- **Owner**: A named object with a single instance. Address retrievable via view functions. Utilizes `signercapability`.
- **Initialization**: Best practices for calling the private entry initialize function.
- **Total Supply Tracking**: Methodologies for tracking the total supply of USDC.
- **Balance Retrieval**: Check how to retrieve balance for primary/secondary fungible stores. Consider if only primary fungible store should be allowed for minters.
- **Contract Pausing**: Strategy for implementing pausing of the contract.
- **Delegated Spending**: Implementation strategies if required from Circle.
- **Rescuer Concept**: Evaluation of the need for a rescuer concept in Aptos.
- **Minter Access to Transfer Ref**: 
  - Evaluate if minter should have access to the transfer_ref.
  - Consider potential attack vectors with current implementation.
- **Master Minter Functions**: 
  - Implement functions as per Circle's master minter case.
  - Translate owner-controller-worker model to Aptos/Move.
- **Blacklisting Addresses**: Strategy for implementing address blacklisting.
- **Master Minter Updates**: Consideration for updating the master minter. Discuss if it can be done through an upgrade since friends can only be declared once.
