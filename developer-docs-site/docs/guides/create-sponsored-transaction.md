# Create a sponsored transaction
[Sponsored transactions](../move/sponsored-transactions.md) in Aptos allow an account other than the sender to be the gas payer. In this guide, we will cover the [`gas_fee_payer`](https://github.com/aptos-labs/aptos-core/blob/81360f302d63d875cb0e9c41b9934838edf57366/ecosystem/typescript/sdk/examples/typescript/gas_fee_payer.ts) example and demonstrate how to create a sponsored transaction using the TypeScript SDK.

## Prerequisites
Complete the following steps before you run the example:
- Install the [TypeScript SDK](https://aptos.dev/sdks/ts-sdk/index)
- Install the [Aptos CLI](https://aptos.dev/tools/aptos-cli)
- Clone the `aptos-core` [Github repo](https://aptos.dev/guides/building-from-source/#clone-the-aptos-core-repo)

## How to create a sponsored transaction
In the example, we use the `createTokenWithFeePayer` function to allow `account` to create a token while `feePayer` (i.e., the sponsor) pays for the gas fees.

```
async function createTokenWithFeePayer(
  feePayer: AptosAccount,
  account: AptosAccount,
  collectionName: string,
  name: string,
  description: string,
  supply: number,
  uri: string,
  max: BCS.AnyNumber,
  royalty_payee_address?: MaybeHexString,
  royalty_points_denominator?: number,
  royalty_points_numerator?: number,
  property_keys?: Array<string>,
  property_values?: Array<string>,
  property_types?: Array<string>,
  extraArgs?: OptionalTransactionArgs,
): Promise<string> {
  const payload: Types.EntryFunctionPayload = {
    function: "0x3::token::create_token_script",
    type_arguments: [],
    arguments: [
      collectionName,
      name,
      description,
      supply,
      max,
      uri,
      royalty_payee_address,
      royalty_points_denominator,
      royalty_points_numerator,
      [false, false, false, false, false],
      property_keys,
      getPropertyValueRaw(property_values, property_types),
      property_types,
    ],
  };
```

The sponsored transaction is initiated with the sender, transaction payload, and sponsor (i.e., fee payer) account.
```
  const feePayerTxn = await client.generateFeePayerTransaction(account.address().hex(), payload, feePayer.address());
```

The sender and sponsor need to sign the transaction.
```
  const senderAuthenticator = await client.signMultiTransaction(account, feePayerTxn);
  const feePayerAuthenticator = await client.signMultiTransaction(feePayer, feePayerTxn);
```

Finally, the transaction is submitted to the network.
```
  const txn = await client.submitFeePayerTransaction(feePayerTxn, senderAuthenticator, feePayerAuthenticator);
```

Run the example to see how this process in action:
1. Open Terminal
2. Navigate to the TypeScript SDK examples directory: `cd aptos-core\ecosystem\typescript\sdk\examples\typescript`
3. Install the necessary dependencies: `npm install`
4. Run the example script: `npm run gas_fee_payer`
5. Follow prompts and review the output.
6. 

When running the example, this portion demonstrates the creation of a token on Alice's account while Bob pays for the fee

```
const tokenName = "Alice Token";
  txnHash = await createTokenWithFeePayer(
    bob,
    alice,
    collectionName,
    tokenName,
    "Alice's simple token",
    1,
    "https://aptos.dev/img/nyan.jpeg",
    1000,
    alice.address(),
    0,
    0,
    ["key"],
    ["2"],
    ["u64"],
  );
```

This portion demonstrates the transfer of `Token` from Alice's account to Bob's account with Bob paying the fee.
```
  console.log("\n=== Alice sent a transaction to send the token to Bob while Bob paid the gas fee ===");
  txnHash = await tokenClient.directTransferTokenWithFeePayer(
    alice,
    bob,
    alice.address(),
    collectionName,
    tokenName,
    1,
    bob,
    propertyVersion,
    undefined,
  );
```