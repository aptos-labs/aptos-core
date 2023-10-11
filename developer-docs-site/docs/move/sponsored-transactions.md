Sponsored transactions in Aptos allow an account other than the sender to be the gas payer, enabling the deduction of gas from a separately specified Move address upon submitting the transaction to the blockchain. This occurs in a secure manner without giving access to the transaction's signer for any other purpose. In other words, the transaction will be executed exactly the same regardless of who pays for the gas. 

Sponsored transactions are a type of multi-agent transaction that uses the `FeePayer` implementation in the Aptos MoveVM, an option that can be used when building a multi-agent transaction. Multi-agent transactions contain multiple, distinct signatures. One signature for each on-chain Aptos account (i.e., one primary and 0-N secondary signers) defined in the transaction. This allows for things like dually-attested on-chain transactions, atomic swaps, etc.

Sponsored transactions can be constructed in two ways:
1. [Be the sponsor](#be-the-sponsor) - The sponsor creates a transaction for the user and specifies themselves (i.e., the sender) as the gas payer, resulting in the user receiving a  request to initiate a transaction in which the gas has been prepaid.
2. [Request a sponsor](#request-a-sponsor) - The user creates a transaction and specifies a different gas fee payer (i.e., someone other than the sender, such as the receipient or a third party), resulting in the gas fee payer receiving a request for them to cover the transaction gas cost before the transaction can be initiated.

## Benefits of sponsored transactions
Before the introduction of sponsored transactions, transaction gas was always deducted from the sender (e.g., user of a dapp). While it was possible to achieve a result similar to a sponsored transaction through a workaround by constructing multi-agent transactions with a custom proxy Move function, this approach was cumbersome and required both custom on-chain and off-chain code. 

Additionally, the workaround used the gas payer's nonce, creating a bottleneck for scaling gas-paying operations where the gas payer account could be paying gas for many transactions from many different users.

The need for proper sponsored transactions funcitonality arises from common and high impact use cases:
- Developers that want to pay for gas fees on behalf of its dapp users to reduce barrier of entry and friction while using the dapp.
- Simplified asset management across many addresses by avoiding the need for each account to have its own APT to cover gas costs.

:::tip Keep in mind typical smart contract risks while utilizing this feature, such as bugs or vulnerabilities in the VM (gas charging) and Move (transaction prologue/epilogue) changes. :::

## Sponsored transaction data structure
`MultiAgentWithFeePayer` is wrapped within the `RawTransactionWithData` data construct (an extended version of `SignedTransaction` that includes the data necessary for a sponosred transaction). See mod.rs for the source code.

```
pub enum RawTransactionWithData {
    MultiAgentWithFeePayer {
        raw_txn: RawTransaction,
        secondary_signer_addresses: Vec<AccountAddress>,
        fee_payer_address: AccountAddress,
    }
```
-  `raw_txn` - Creates a `RawTransaction` (the portion of a transaction that a client signs).
-  `secondary_signer_addresses` - The other involved parties' addresses. There can be 0-N of these.
-  `fee_payer_address` - The address of the paying party.


The `sign_fee_payer` public function accepts private keys for the sender, secondary signers, and gas fee payer to respectively sign the `RawTransaction` and generate a `SignedTransaction`.

```
    pub fn sign_fee_payer(
        self,
        sender_private_key: &Ed25519PrivateKey,
        secondary_signers: Vec<AccountAddress>,
        secondary_private_keys: Vec<&Ed25519PrivateKey>,
        fee_payer_address: AccountAddress,
        fee_payer_private_key: &Ed25519PrivateKey,
    ) -> Result<SignatureCheckedTransaction> {
        let message = RawTransactionWithData::new_fee_payer(
            self.clone(),
            secondary_signers.clone(),
            fee_payer_address,
        );

        // ............... //

        Ok(SignatureCheckedTransaction(
            SignedTransaction::new_fee_payer(
                self,
                sender_authenticator,
                secondary_signers,
                secondary_authenticators,
                fee_payer_address,
                fee_payer_authenticator,
            ),
        ))
    }
```

After a `SignedTransaction` is submitted to the Aptos blockchain, it undergoes validation where the sender, secondary signer, and  signature is checked against the transaction hash for correctness. Furthermore, a SHA-3 hash from each signature is compared against the `AuthenticationKey` stored in the signer's account address to ensure validity. See the `FeePayer` implemenation wrapped in the `TransactionAuthenticator` data construct below, or authenticator.rs for source code.

```
pub enum TransactionAuthenticator {
    FeePayer {
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
        fee_payer_address: AccountAddress,
        fee_payer_signer: AccountAuthenticator,
    },
}
```
- `sender` - The signature of the sender.
- `secondary_signer` - The other involved parties' addresses.
- `secondary_signers` - The associated signatures, in the same order as the secondary addresses.
- `fee_payer_address` -  The address of the paying party (i.e., sponsor).
- `fee_payer_signer` - The signature of the fee payer (i.e., sponsor).

## Pathways to a sponsored transaction
Sponsored transactions can be initiated by either users that'd like to request a sponsor, or sponsors that'd like to prepay for a user. To see an example of how to implement sponosored transaction functionality in your dapp via the Aptos SDK, read the [Sponsored Transaction](../guides/create-sponsored-transaction.md) SDK guide.

### Initiate a transaction as the sponsor
The flow of a sponsor-initiated sponsored transasction (i.e. proactively cover the gas cost for another account):
1. The sponsor initializes `RawTransactionWithData::MultiAgentWithFeePayer` by signing as the sender, and specifying the addresses for the secondary signer(s) and sponsor.
2. The sponsor sends the `RawTransaction` to the user.
3. The user reviews and signs the `RawTransaction`.
4. The user submits the `SignedTransaction` to the network or sends it back for the sponsor to submit it.

### Initiate a transaction as the user
The flow of a user-initiated sponsored transaction (i.e. asking another account to cover the gas cost):
1. The user initializes `RawTransactionWithData::MultiAgentWithFeePayer` by signing as the sender, and specifying the secondary signer and sponsor addresses.
2. The user sends the `RawTransaction` to the sponsor.
3. The sponsor reviews and signs the `RawTransaction`.
4. The sponsor submits the `SignedTransaction` to the network or sends it back for the user to submit it.