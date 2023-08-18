import {
  AptosAccount,
  FaucetClient,
  Network,
  Provider,
  HexString,
  TxnBuilderTypes,
  BCS,
  Types,
  TransactionBuilder,
} from "aptos";
import assert from "assert";

const ED25519_ACCOUNT_SCHEME = 0;
const MULTI_ED25519_ACCOUNT_SCHEME = 1;

class MultiSigAccountCreationWithAuthKeyRevocationMessage {
  public readonly moduleAddress: TxnBuilderTypes.AccountAddress = TxnBuilderTypes.AccountAddress.CORE_CODE_ADDRESS;
  public readonly moduleName: string = "multisig_account";
  public readonly structName: string = "MultisigAccountCreationWithAuthKeyRevocationMessage";
  public readonly functionName: string = "create_with_existing_account_and_revoke_auth_key";

  constructor(
    public readonly chainId: number,
    public readonly multiSigAddress: TxnBuilderTypes.AccountAddress,
    public readonly sequenceNumber: number,
    public readonly owners: Array<TxnBuilderTypes.AccountAddress>,
    public readonly numSignaturesRequired: number,
  ) {}

  serialize(serializer: BCS.Serializer): void {
    this.moduleAddress.serialize(serializer);
    serializer.serializeStr(this.moduleName);
    serializer.serializeStr(this.structName);
    serializer.serializeU8(this.chainId);
    this.multiSigAddress.serialize(serializer);
    serializer.serializeU64(this.sequenceNumber);
    serializer.serializeU32AsUleb128(this.owners.length);
    this.owners.forEach((owner) => owner.serialize(serializer));
    serializer.serializeU64(this.numSignaturesRequired);
  }
}

//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////
//                                                                                      //
//                              Demonstration of e2e flow                               //
//                                                                                      //
//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////
/*
  * This example demonstrates how to convert a MultiEd25519 account to a MultiSig account and revoke its auth key using the `0x1::multisig_account` module.
    1. Initialize N accounts and fund them.
    2. Initialize a MultiEd25519 account with the created accounts as owners and a signature threshold K as NUM_SIGNATURES_REQUIRED
      - See: https://aptos.dev/concepts/accounts/#multi-signer-authentication for more information on MultiEd25519 accounts.
    3. Create a proof struct for at minimum K of the N accounts to sign.
    4. Gather the signatures from the accounts.
    5. Assemble a MultiEd25519 signed proof struct with the gathered signatures.
    6. Call the `0x1::multisig_account::create_with_existing_account_and_revoke_auth_key` function with the assembled proof struct and other logistical information
      - Because the function requires a signed proof by the MultiEd25519 account, it does not require or check the signer, meaning anyone can submit the transaction
        with the proof struct.
      - We submit it as a randomly generated account here to convey this.
    7. The transaction will be executed and the following occurs on chain:
      a. The MultiEd25519 account is converted into a MultiSig account.
      b. The resulting account can from then on be used as a MultiSig account, potentially with new owners and/or a new minimum signature threshold.
      c. The original MultiEd25519 account has its authentication key rotated, handing over control to the `0x1::multisig_account` contract.
*/
const main = async () => {
  const provider = new Provider(Network.DEVNET);
  const faucetClient = new FaucetClient(provider.aptosClient.nodeUrl, "https://faucet.devnet.aptoslabs.com");
  const NUM_SIGNATURES_REQUIRED = 3;

  // Step 1.
  // Initialize N accounts and fund them. See: https://aptos.dev/concepts/accounts/#multi-signer-authentication
  // Works with any # of addresses, you just need to change NUM_SIGNATURES_REQUIRED and the signingAddresses
  const account1 = new AptosAccount();
  const account2 = new AptosAccount();
  const account3 = new AptosAccount();

  await faucetClient.fundAccount(account1.address(), 100_000_000);
  await faucetClient.fundAccount(account2.address(), 100_000_000);
  await faucetClient.fundAccount(account3.address(), 100_000_000);

  const accounts = [account1, account2, account3];
  const accountAddresses = accounts.map((acc) => TxnBuilderTypes.AccountAddress.fromHex(acc.address()));

  // If the signing accounts are a subset of the original accounts in the actual e2e flow, we'd use this to track who's actually signing
  const signingAccounts = [account1, account2, account3];
  const signingAddresses = signingAccounts.map((acc) => TxnBuilderTypes.AccountAddress.fromHex(acc.address()));

  // Step 2.
  // Initialize a MultiEd25519 account with the created accounts as owners and a signature threshold K as NUM_SIGNATURES_REQUIRED
  await initializeMultiEd25519(faucetClient, accounts, NUM_SIGNATURES_REQUIRED);

  const publicKeys = accounts.map((acc) => new TxnBuilderTypes.Ed25519PublicKey(acc.signingKey.publicKey));
  const multiSigPublicKey = new TxnBuilderTypes.MultiEd25519PublicKey(publicKeys, NUM_SIGNATURES_REQUIRED);
  const authKey = TxnBuilderTypes.AuthenticationKey.fromMultiEd25519PublicKey(multiSigPublicKey);
  const multiSigAddress = TxnBuilderTypes.AccountAddress.fromHex(authKey.derivedAddress());

  const sequenceNumber = Number((await provider.getAccount(multiSigAddress.toHexString())).sequence_number);
  const chainId = Number(await provider.getChainId());

  // Step 3.
  // Create a proof struct for at minimum K of the N accounts to sign
  const proofStruct = new MultiSigAccountCreationWithAuthKeyRevocationMessage(
    chainId,
    multiSigAddress,
    sequenceNumber,
    accountAddresses,
    NUM_SIGNATURES_REQUIRED,
  );

  // Step 4.
  // Gather the signatures from the accounts
  // In an e2e dapp example, you'd be getting these from each account/client with a wallet prompt to sign a message.
  const bcsSerializedStruct = BCS.bcsToBytes(proofStruct);
  const structSig1 = account1.signBuffer(bcsSerializedStruct);
  const structSig2 = account2.signBuffer(bcsSerializedStruct);
  const structSig3 = account3.signBuffer(bcsSerializedStruct);
  const structSignatures = [structSig1, structSig2, structSig3].map((sig) => sig.toUint8Array());

  // Step 5.
  // Assemble a MultiEd25519 signed proof struct with the gathered signatures.
  // This represents the multisig signed struct by all owners. This is used as proof of authentication by the overall MultiEd25519 account, since there's no signer
  // checked in the entry function we're using.
  const multiSigStruct = createMultiSigStructFromSignedStructs(accountAddresses, signingAddresses, structSignatures);

  // Create test metadata for the multisig account post-creation
  const metadataKeys = ["key 123", "key 456", "key 789"];
  const metadataValues = [new Uint8Array([1, 2, 3]), new Uint8Array([4, 5, 6]), new Uint8Array([7, 8, 9])];

  // Pack the signed multi-sig struct into the entry function payload with the number of signatures required + multisig account info
  const entryFunctionPayload = createWithExistingAccountAndRevokeAuthKeyPayload(
    multiSigAddress,
    multiSigPublicKey,
    multiSigStruct,
    accountAddresses,
    NUM_SIGNATURES_REQUIRED,
    metadataKeys,
    metadataValues,
  );

  // Step 6.
  // Call the `0x1::multisig_account::create_with_existing_account_and_revoke_auth_key` function
  //   Since you've already authenticated the signed struct message with the required K of N accounts, you do not need to construct a MultiEd25519 authenticated signature.
  //   You can submit the transaction as literally any account, because the entry function does not check the sender. The transaction is validated with the multisig signed struct.
  const randomAccount = new AptosAccount();
  await faucetClient.fundAccount(randomAccount.address(), 100_000_000);

  // The sender here is essentially just paying for the gas fees.
  const txn = await provider.generateSignSubmitTransaction(randomAccount, entryFunctionPayload, {
    expireTimestamp: BigInt(Math.floor(Date.now() / 1000) + 60),
  });

  // Step 7.
  // Wait for the transaction to complete, then observe and assert that the authentication key for the original MultiEd25519 account
  // has been rotated and all capabilities revoked.
  const txnInfo = await provider.waitForTransactionWithResult(txn);
  printRelevantTxInfo(txnInfo as Types.UserTransaction);
  assertChangesAndPrint(provider, multiSigAddress, sequenceNumber, false, metadataKeys, metadataValues);
};

//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////
//                                                                                      //
//                               Helper/utility functions                               //
//                                                                                      //
//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////

// For clarification, the process of creating a MultiEd25519 account is:
//  1. Create or specify N accounts. We will use their public keys to generate the MultiEd25519 account.
//  2. Create a MultiEd25519 public key with the N public keys and a signature threshold K. K must be <= N.
//      NOTE: A public key is different from an account's address. It can be always derived from an account's authentication key or private key, not necessarily its address.
//  3. Create a MultiEd25519 authentication key with the MultiEd25519 public key.
//      You can then derive the address from the authentication key.
//  4. Fund the derived MultiEd25519 account at the derived address.
//
// Funds and thus creates the derived MultiEd25519 account and prints out the derived address, authentication key, and public key.
const initializeMultiEd25519 = async (
  faucetClient: FaucetClient,
  accounts: Array<AptosAccount>,
  numSignaturesRequired: number,
): Promise<Array<string>> => {
  const multiSigPublicKey = new TxnBuilderTypes.MultiEd25519PublicKey(
    accounts.map((acc) => new TxnBuilderTypes.Ed25519PublicKey(acc.signingKey.publicKey)),
    numSignaturesRequired,
  );

  const multiSigAuthKey = TxnBuilderTypes.AuthenticationKey.fromMultiEd25519PublicKey(multiSigPublicKey);
  const multiSigAddress = multiSigAuthKey.derivedAddress();

  // Note that a MultiEd25519's public and private keys are simply the concatenated corresponding key values of the original owners.
  console.log("\nMultiEd25519 account information:");
  console.log({
    MultiEd25519Address: multiSigAddress.toString(),
    MultiEd25519PublicKey: HexString.fromUint8Array(multiSigPublicKey.toBytes()).toString(),
  });

  return await faucetClient.fundAccount(multiSigAddress.toString(), 100_000_000);
};

// Helper function to create the bitmap from the difference between the original addresses at creation vs the current signing addresses
// NOTE: The originalAddresses MUST be in the order that was used to create the MultiEd25519 account originally.
const createBitmapFromDiff = (
  originalAddresses: Array<TxnBuilderTypes.AccountAddress>,
  signingAddresses: Array<TxnBuilderTypes.AccountAddress>,
): Uint8Array => {
  const signersSet = new Set(signingAddresses.map((addr) => addr.toHexString()));
  const bits = originalAddresses
    .map((addr) => addr.toHexString())
    .map((item, index) => (signersSet.has(item) ? index : -1))
    .filter((index) => index !== -1);

  // Bitmap masks which public key has signed transaction.
  // See https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.MultiEd25519Signature.html#createBitmap
  return TxnBuilderTypes.MultiEd25519Signature.createBitmap(bits);
};

// This is solely used to create the entry function payload for the 0x1::multisig_account::create_with_existing_account_and_revoke_auth_key function
// The multiSignedStruct is constructed with the `signStructForMultiSig` function.
const createWithExistingAccountAndRevokeAuthKeyPayload = (
  multiSigAddress: TxnBuilderTypes.AccountAddress,
  multiSigPublicKey: TxnBuilderTypes.MultiEd25519PublicKey,
  multiSignedStruct: Uint8Array,
  newOwners: Array<TxnBuilderTypes.AccountAddress>,
  newNumSignaturesRequired: number,
  metadataKeys: Array<string>,
  metadataValues: Array<Uint8Array>,
): TxnBuilderTypes.TransactionPayloadEntryFunction => {
  assert(metadataKeys.length == metadataValues.length, "Metadata keys and values must be the same length.");
  return new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      `0x1::multisig_account`,
      "create_with_existing_account_and_revoke_auth_key",
      [],
      [
        BCS.bcsToBytes(multiSigAddress),
        BCS.serializeVectorWithFunc(
          newOwners.map((o) => o.address),
          "serializeFixedBytes",
        ),
        BCS.bcsSerializeUint64(newNumSignaturesRequired),
        BCS.bcsSerializeU8(MULTI_ED25519_ACCOUNT_SCHEME),
        BCS.bcsSerializeBytes(multiSigPublicKey.toBytes()),
        BCS.bcsSerializeBytes(multiSignedStruct),
        BCS.serializeVectorWithFunc(metadataKeys, "serializeStr"),
        BCS.serializeVectorWithFunc(metadataValues, "serializeBytes"),
      ],
    ),
  );
};

// We create the multisig struct by concatenating the individually signed structs together
// Then we append the bitmap at the end.
const createMultiSigStructFromSignedStructs = (
  accountAddresses: Array<TxnBuilderTypes.AccountAddress>,
  signingAddresses: Array<TxnBuilderTypes.AccountAddress>,
  signatures: Array<Uint8Array>,
): Uint8Array => {
  // Flatten the signatures into a single byte array
  let flattenedSignatures = new Uint8Array();
  signatures.forEach((sig) => {
    flattenedSignatures = new Uint8Array([...flattenedSignatures, ...sig]);
  });

  // This is the bitmap indicating which original owners are present as signers. It takes a diff of the original owners and the signing owners and creates the bitmap based on that.
  const bitmap = createBitmapFromDiff(accountAddresses, signingAddresses);

  // Add the bitmap to the end of the byte array
  return new Uint8Array([...flattenedSignatures, ...bitmap]);
};

// Helper function to check all the different resources on chain that should have changed and print them out
const assertChangesAndPrint = async (
  provider: Provider,
  multiEd25519Address: TxnBuilderTypes.AccountAddress,
  sequenceNumber: number,
  submittedAsMultiEd25519: boolean,
  metadataKeys: Array<string>,
  metadataValues: Array<Uint8Array>,
) => {
  // Query the account resources on-chain
  const accountResource = await provider.getAccountResource(multiEd25519Address.toHexString(), "0x1::account::Account");
  const data = accountResource?.data as any;

  // Check that the authentication key of the original MultiEd25519 account was rotated
  // Normalize the inputs and then convert them to a string for comparison
  const authKey = TxnBuilderTypes.AccountAddress.fromHex(data.authentication_key).toHexString().toString();
  const zeroAuthKey = TxnBuilderTypes.AccountAddress.fromHex("0x0").toHexString().toString();
  const authKeyRotated = authKey === zeroAuthKey;

  // Sequence number only increments if MultiEd was the signer
  const expectedSequenceNumber = submittedAsMultiEd25519 ? sequenceNumber + 1 : sequenceNumber + 0;

  // Check the rotation/signer capability offers. They should have been revoked if there were any outstanding offers
  const rotationCapabilityOffer = data.rotation_capability_offer.for.vec as Array<any>;
  const signerCapabilityOffer = data.signer_capability_offer.for.vec as Array<any>;

  // Check that the metadata keys and values were correctly stored at the multisig's account address
  const multisigAccountResource = await provider.getAccountResource(
    multiEd25519Address.toHexString(),
    "0x1::multisig_account::MultisigAccount",
  );

  const metadata = (multisigAccountResource?.data as any).metadata.data as Array<any>;
  const onChainMetadataValues = metadata.map((m) => new HexString(m.value).toUint8Array());

  console.log(`\nMetadata added to MultiSig Account:`);
  console.log(metadata);

  // Assert our expectations about the metadata key/value map
  onChainMetadataValues.forEach((v, i) => {
    assert(
      v.length === metadataValues[i].length,
      `Incorrect length. Input ${metadataValues[i].length} but on-chain length is ${v.length}`,
    );
    (v as unknown as Array<number>).forEach((vv, ii) => {
      assert(
        Number(vv) === Number(metadataValues[i][ii]),
        `Incorrect value. Input ${metadataValues[i][ii]} but on-chain value is ${vv}`,
      );
    });
  });
  const onChainMetadataKeys = metadata.map((m) => m.key);
  assert(
    onChainMetadataKeys.length === metadataKeys.length,
    `Incorrect length. Input ${metadataKeys.length} but on-chain length is ${onChainMetadataKeys.length}`,
  );
  onChainMetadataKeys.forEach((k, i) => {
    assert(k === metadataKeys[i], `Incorrect key. Input ${metadataKeys[i]} but on-chain key is ${k}`);
  });

  // Assert our expectations about the account resources
  assert(Number(data.sequence_number) === expectedSequenceNumber, "Incorrect sequence number.");
  assert(authKeyRotated, "nAuthentication key was not rotated.");
  assert(rotationCapabilityOffer.length == 0);
  assert(signerCapabilityOffer.length == 0);

  // Print any relevant account resource info
  console.log(`\nAuthentication key was rotated successfully:`);
  console.log({
    authentication_key: data.authentication_key,
    sequence_number: Number(data.sequence_number),
    rotation_capability_offer: rotationCapabilityOffer,
    signer_capability_offer: signerCapabilityOffer,
  });
};

const printRelevantTxInfo = (txn: Types.UserTransaction): void => {
  const signatureType = txn.signature?.type;
  let signatureTypeMessage = "";
  switch (signatureType) {
    case "ed25519_signature":
      signatureTypeMessage = "a MultiEd25519 account.";
      break;
    case "multi_ed_25519_signature":
      signatureTypeMessage = "an Ed25519 account.";
      break;
    default:
      signatureTypeMessage = "a different signature type.";
  }
  console.log(txn.payload);
  // Print the relevant transaction response information.
  console.log(`\nSubmitted transaction response as ${signatureType}:`);
  console.log({
    version: txn.version,
    hash: txn.hash,
    success: txn.success,
    vm_status: txn.vm_status,
    sender: txn.sender,
    expiration_timestamp_secs: txn.expiration_timestamp_secs,
    payload: txn.payload,
    signature: txn.signature,
    events: txn.events,
    timestamp: txn.timestamp,
  });
};

main();
