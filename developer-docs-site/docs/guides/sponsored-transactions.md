# Sponsored Transactions

As outlined in [AIP-39](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-39.md),sponsored transactions allow one account to pay the fees associated with executing a transaction for another account. Sponsored transactions simplify the process for onboarding users into applications by allowing the application to cover all associated fees for interacting with the Aptos blockchain. Here are two examples:
* [MerkleTrade](https://merkle.trade/) offers low cost trading to those with Ethereum wallets by creating an Aptos wallet for users and covering all transaction fees so that the user does not need to acquire utility tokens for Aptos.
* Community engagement applications like [Graffio](https://medium.com/aptoslabs/graffio-web3s-overnight-sensation-81a6cf18b626) offered to cover transaction fees for custodial accounts to support the collaborative drawing application for those without wallets.

## Process Overview

The process for sending a sponsored transaction follows:
* The sender of the transaction determines upon an operation, as defined by a `RawTransaction`.
* The sender generates a `RawTransactionWithData::MultiAgentWithFeePayer` structure
  * Prior to the framework 1.8 release, this must contain the fee payer's address.
  * After framework release 1.8, this can optionally be set to `0x0`.
* (Optionally) the sender aggregates signatures from other signers.
* The sender can forward the signed transaction to the fee payer to sign and forward it to the blockchain.
* Upon execution of the transaction, the sequence number of the sender account is incremented, all gas fees are deducted from the gas fee payer, and all refunds are sent to the gas fee payer.

Alternatively, if the fee payer knows the operation and all signers involved, the fee payer could generate and sign the transaction and send it back to the other signers to sign.

## Technical Details

In Aptos, a sponsored transaction reuses the same SignedTransaction as any other user transaction:
```rust
pub struct SignedTransaction {
    /// The raw transaction
    raw_txn: RawTransaction,

    /// Public key and signature to authenticate
    authenticator: TransactionAuthenticator,
}
```

The difference is in the `TransactionAuthenticator`, which stores the authorization from the fee payer of the transaction to extract utility fees from their account:
```rust
pub enum TransactionAuthenticator {
...
    /// Optional Multi-agent transaction with a fee payer.
    FeePayer {
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
        fee_payer_address: AccountAddress,
        fee_payer_signer: AccountAuthenticator,
    },
...
}
```

To prepare a sponsored transaction for an account, the account must first exist on-chain. This is a requirement that is being removed with the 1.8 framework release.

As of the 1.8 framework release, an account does not need to exist on-chain. However, the first transaction for an account requires enough gas to not only execute the transaction and cover the costs associated with account creation, even if an account already exists. Future improvements to the account model intend to eliminate this requirement.

During signing of the transaction, all parties sign the following:
```rust
pub enum RawTransactionWithData {
...
    MultiAgentWithFeePayer {
        raw_txn: RawTransaction,
        secondary_signer_addresses: Vec<AccountAddress>,
        fee_payer_address: AccountAddress,
    },
}
```

Prior to framework release 1.8, all signers were required to know the actual fee payer address prior to signing. As of framework release 1.8, signers can optionally set the address to `0x0` and only the fee payer must sign with their address set.

## SDK Support

Currently, there are two demonstrations of sponsored transactions:
* The Python SDK has an example in [fee_payer_transfer_coin.py](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/fee_payer_transfer_coin.py).
* The Rust SDK has a test case in [the API tests](https://github.com/aptos-labs/aptos-core/blob/0a62e54e13bc5da604ceaf39efed5c012a292078/api/src/tests/transactions_test.rs#L255).
