import { AptosAccount, FaucetClient, Network, Provider, HexString, TxnBuilderTypes, BCS, Types } from "aptos";
import assert from "assert";

const CORE_CODE_ADDRESS = new HexString("0x0000000000000000000000000000000000000000000000000000000000000001");
const ED25519_ACCOUNT_SCHEME = 0;

// Works for both SignerCapabilityOffer and RotationCapabilityOffer
// Except the structName fields are different and the chainId isn't used in SignerCapabilityOffer
type CapabilityOfferProofChallengeV2 = {
  accountAddress: TxnBuilderTypes.AccountAddress;
  moduleName: string;
  structName: string;
  sequenceNumber: number;
  sourceAddress: TxnBuilderTypes.AccountAddress;
  recipientAddress: TxnBuilderTypes.AccountAddress;
  chainId?: number; // not used in SignerCapabilityOffer
};

const createAndFundAliceAndBob = async (
  faucetClient: FaucetClient,
): Promise<{ alice: AptosAccount; bob: AptosAccount }> => {
  console.log(`\n---------  Creating and funding new accounts for Bob & Alice  ---------\n`);
  const alice = new AptosAccount();
  const bob = new AptosAccount();
  await faucetClient.fundAccount(alice.address(), 100_000_000);
  await faucetClient.fundAccount(bob.address(), 100_000_000);
  console.log({
    alice: alice.address().toString(),
    bob: bob.address().toString(),
  });
  return {
    alice,
    bob,
  };
};

(async () => {
  const provider = new Provider(Network.DEVNET);
  const faucetClient = new FaucetClient(provider.aptosClient.nodeUrl, "https://faucet.devnet.aptoslabs.com");
  const chainId = await provider.getChainId();

  const { alice, bob } = await createAndFundAliceAndBob(faucetClient);
  const aliceAccountAddress = TxnBuilderTypes.AccountAddress.fromHex(alice.address());
  const bobAccountAddress = TxnBuilderTypes.AccountAddress.fromHex(bob.address());
  const moduleAddress = TxnBuilderTypes.AccountAddress.fromHex(CORE_CODE_ADDRESS);

  // Offer Alice's rotation capability to Bob
  {
    // Construct the RotationCapabilityOfferProofChallengeV2 struct
    const rotationCapabilityOffer: CapabilityOfferProofChallengeV2 = {
      accountAddress: moduleAddress,
      moduleName: "account",
      structName: "RotationCapabilityOfferProofChallengeV2",
      sequenceNumber: Number((await provider.getAccount(alice.address())).sequence_number),
      sourceAddress: aliceAccountAddress,
      recipientAddress: bobAccountAddress,
      chainId,
    };

    console.log(`\n---------------  RotationCapabilityOfferProofChallengeV2 --------------\n`);

    // Sign the BCS-serialized struct, submit the transaction, and wait for the result.
    const res = await signStructAndSubmitTransaction(
      provider,
      alice,
      "offer_rotation_capability",
      rotationCapabilityOffer,
    );

    // Print the relevant transaction submission info
    const { hash, version, success, payload } = res;
    console.log("Submitted transaction results:");
    console.log({ hash, version, success, payload });

    // Query Alice's Account resource on-chain to verify that she has offered the rotation capability to Bob
    console.log("\nChecking Alice's account resources to verify the rotation capability offer is for Bob...");
    const { data } = await provider.getAccountResource(alice.address(), "0x1::account::Account");
    const offerFor = (data as any).rotation_capability_offer.for.vec[0];

    console.log({ rotation_capability_offer: { for: offerFor } });
    assert(offerFor.toString() == bob.address().toString(), "Bob's address should be in the rotation capability offer");
    console.log("...success!\n");
  }

  // Offer Alice's signer capability to Bob
  {
    // Construct the SignerCapabilityOfferProofChallengeV2 struct
    const signerCapabilityOffer: CapabilityOfferProofChallengeV2 = {
      accountAddress: moduleAddress,
      moduleName: "account",
      structName: "SignerCapabilityOfferProofChallengeV2",
      sequenceNumber: Number((await provider.getAccount(alice.address())).sequence_number),
      sourceAddress: aliceAccountAddress,
      recipientAddress: bobAccountAddress,
      // Note no chainId, the signer capability offer doesn't require it. We leave it undefined
    };

    console.log(`\n---------------  SignerCapabilityOfferProofChallengeV2 ---------------\n`);

    // Sign the BCS-serialized struct, submit the transaction, and wait for the result.
    const res = await signStructAndSubmitTransaction(provider, alice, "offer_signer_capability", signerCapabilityOffer);

    // Print the relevant transaction submission info
    const { hash, version, success, payload } = res;
    console.log("Submitted transaction results:");
    console.log({ hash, version, success, payload });

    // Query Alice's Account resource on-chain to verify that she has offered the signer capability to Bob
    console.log("\nChecking Alice's account resources to verify the signer capability offer is for Bob...");
    const { data } = await provider.getAccountResource(alice.address(), "0x1::account::Account");
    const offerFor = (data as any).signer_capability_offer.for.vec[0];

    console.log({ signer_capability_offer: { for: offerFor } });
    assert(offerFor.toString() == bob.address().toString(), "Bob's address should be in the signer capability offer\n");
    console.log("...success!\n");
  }
})();

const signStructAndSubmitTransaction = async (
  provider: Provider,
  signer: AptosAccount,
  funcName: string,
  struct: CapabilityOfferProofChallengeV2,
): Promise<any> => {
  // The proof bytes are just the individual BCS serialized
  // data concatenated into a single byte array.
  // Note that the proof bytes must be constructed in this specific order: the order of the struct data on-chain.
  const proofBytes = new Uint8Array([
    ...BCS.bcsToBytes(struct.accountAddress),
    ...BCS.bcsSerializeStr(struct.moduleName),
    ...BCS.bcsSerializeStr(struct.structName),
    ...(struct.chainId ? BCS.bcsSerializeU8(struct.chainId) : []),
    ...BCS.bcsSerializeUint64(struct.sequenceNumber),
    ...BCS.bcsToBytes(struct.sourceAddress),
    ...BCS.bcsToBytes(struct.recipientAddress),
  ]);

  // This is the actual signature of the struct.
  const signedMessage = signer.signBuffer(proofBytes).toUint8Array();

  // Note the hard-coded account scheme, this would not work for a MultiEd25519 account.
  const payload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      `0x1::account`,
      funcName,
      [],
      [
        BCS.bcsSerializeBytes(signedMessage),
        BCS.bcsSerializeU8(ED25519_ACCOUNT_SCHEME),
        BCS.bcsSerializeBytes(signer.pubKey().toUint8Array()),
        BCS.bcsToBytes(struct.recipientAddress),
      ],
    ),
  );

  const txn = await provider.generateSignSubmitTransaction(signer, payload);
  return (await provider.waitForTransactionWithResult(txn)) as Types.UserTransaction;
};
