import { AptosAccount, FaucetClient, Network, Provider, HexString, TxnBuilderTypes, BCS, Types } from "aptos";
import assert from "assert";

const ED25519_ACCOUNT_SCHEME = 0;

class SignerCapabilityOfferProofChallengeV2 {
  public readonly moduleAddress: TxnBuilderTypes.AccountAddress = TxnBuilderTypes.AccountAddress.CORE_CODE_ADDRESS;
  public readonly moduleName: string = "account";
  public readonly structName: string = "SignerCapabilityOfferProofChallengeV2";
  public readonly functionName: string = "offer_signer_capability";

  constructor(
    public readonly sequenceNumber: number,
    public readonly sourceAddress: TxnBuilderTypes.AccountAddress,
    public readonly recipientAddress: TxnBuilderTypes.AccountAddress,
  ) {}

  serialize(serializer: BCS.Serializer): void {
    this.moduleAddress.serialize(serializer);
    serializer.serializeStr(this.moduleName);
    serializer.serializeStr(this.structName);
    serializer.serializeU64(this.sequenceNumber);
    this.sourceAddress.serialize(serializer);
    this.recipientAddress.serialize(serializer);
  }
}

class RotationCapabilityOfferProofChallengeV2 {
  public readonly moduleAddress: TxnBuilderTypes.AccountAddress = TxnBuilderTypes.AccountAddress.CORE_CODE_ADDRESS;
  public readonly moduleName: string = "account";
  public readonly structName: string = "RotationCapabilityOfferProofChallengeV2";
  public readonly functionName: string = "offer_rotation_capability";

  constructor(
    public readonly chainId: number,
    public readonly sequenceNumber: number,
    public readonly sourceAddress: TxnBuilderTypes.AccountAddress,
    public readonly recipientAddress: TxnBuilderTypes.AccountAddress,
  ) {}

  serialize(serializer: BCS.Serializer): void {
    this.moduleAddress.serialize(serializer);
    serializer.serializeStr(this.moduleName);
    serializer.serializeStr(this.structName);
    serializer.serializeU8(this.chainId);
    serializer.serializeU64(this.sequenceNumber);
    this.sourceAddress.serialize(serializer);
    this.recipientAddress.serialize(serializer);
  }
}

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

  // Offer Alice's rotation capability to Bob
  {
    // Construct the RotationCapabilityOfferProofChallengeV2 struct
    const rotationCapProof = new RotationCapabilityOfferProofChallengeV2(
      chainId,
      Number((await provider.getAccount(alice.address())).sequence_number), // Get Alice's account's latest sequence number
      aliceAccountAddress,
      bobAccountAddress,
    );

    console.log(`\n---------------  RotationCapabilityOfferProofChallengeV2 --------------\n`);

    // Sign the BCS-serialized struct, submit the transaction, and wait for the result.
    const res = await signStructAndSubmitTransaction(provider, alice, rotationCapProof, ED25519_ACCOUNT_SCHEME);

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
    const signerCapProof = new SignerCapabilityOfferProofChallengeV2(
      Number((await provider.getAccount(alice.address())).sequence_number), // Get Alice's account's latest sequence number
      aliceAccountAddress,
      bobAccountAddress,
    );

    console.log(`\n---------------  SignerCapabilityOfferProofChallengeV2 ---------------\n`);

    // Sign the BCS-serialized struct, submit the transaction, and wait for the result.
    const res = await signStructAndSubmitTransaction(provider, alice, signerCapProof, ED25519_ACCOUNT_SCHEME);

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
  struct: SignerCapabilityOfferProofChallengeV2 | RotationCapabilityOfferProofChallengeV2,
  accountScheme: number = ED25519_ACCOUNT_SCHEME,
): Promise<any> => {
  const bcsStruct = BCS.bcsToBytes(struct);
  const signedMessage = signer.signBuffer(bcsStruct);

  const payload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
    TxnBuilderTypes.EntryFunction.natural(
      `${struct.moduleAddress.toHexString()}::${struct.moduleName}`,
      struct.functionName,
      [],
      [
        BCS.bcsSerializeBytes(signedMessage.toUint8Array()),
        BCS.bcsSerializeU8(accountScheme),
        BCS.bcsSerializeBytes(signer.pubKey().toUint8Array()),
        BCS.bcsToBytes(struct.recipientAddress),
      ],
    ),
  );
  const txnResponse = await provider.generateSignSubmitWaitForTransaction(signer, payload);
  return txnResponse as Types.UserTransaction;
};
