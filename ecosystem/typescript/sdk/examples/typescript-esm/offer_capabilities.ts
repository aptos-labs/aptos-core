import { AptosAccount, FaucetClient, Network, Provider, HexString, TxnBuilderTypes, BCS, Types } from "aptos";
import assert from "assert";

const CORE_CODE_ADDRESS = new HexString("0x0000000000000000000000000000000000000000000000000000000000000001");
const ED25519_ACCOUNT_SCHEME = 0;

// Works for both SignerCapabilityOffer and RotationCapabilityOffer.
// The structName changes and chainId isn't used in SignerCapabilityOffer
type CapabilityOfferProofChallengeV2 = {
  accountAddress: TxnBuilderTypes.AccountAddress;
  moduleName: string;
  structName: string;
  chainId?: number; // not used in SignerCapabilityOffer
  sequenceNumber: number;
  sourceAddress: TxnBuilderTypes.AccountAddress;
  recipientAddress: TxnBuilderTypes.AccountAddress;
};

const createAndFundAliceAndBob = async (
  faucetClient: FaucetClient,
): Promise<{ alice: AptosAccount; bob: AptosAccount }> => {
  console.log(`---------------  Creating and funding a new Bob & Alice  ---------------`);
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

  // Offer rotation capability
  {
    // Create and fund two new accounts
    const { alice, bob } = await createAndFundAliceAndBob(faucetClient);
    // Note that the rotation capability offer needs the chainId
    console.log(`---------------  RotationCapabilityOfferProofChallengeV2 --------------`);
    const { hash, version, success, payload } = await signStructAndSubmitTransaction(
      provider,
      alice,
      "offer_rotation_capability",
      {
        accountAddress: TxnBuilderTypes.AccountAddress.fromHex(CORE_CODE_ADDRESS),
        moduleName: "account",
        structName: "RotationCapabilityOfferProofChallengeV2",
        chainId: await provider.aptosClient.getChainId(),
        sequenceNumber: Number((await provider.getAccount(alice.address())).sequence_number),
        sourceAddress: TxnBuilderTypes.AccountAddress.fromHex(alice.address()),
        recipientAddress: TxnBuilderTypes.AccountAddress.fromHex(bob.address()),
      },
    );
    console.log({ hash, version, success, payload });
    const { data } = await provider.aptosClient.getAccountResource(alice.address(), "0x1::account::Account");
    const offerFor = (data as any).rotation_capability_offer.for.vec[0];
    console.log({ rotation_capability_offer: { for: offerFor }, bob: bob.address().toString() });
    assert(offerFor.toString() == bob.address().toString(), "Bob's address should be in the rotation capability offer");
  }

  // Offer signer capability
  {
    // Create and fund two new accounts
    const { alice, bob } = await createAndFundAliceAndBob(faucetClient);
    // Note that the signer capability offer doesn't require the chainId
    console.log(`---------------  SignerCapabilityOfferProofChallengeV2 ---------------`);
    const { hash, version, success, payload } = await signStructAndSubmitTransaction(
      provider,
      alice,
      "offer_signer_capability",
      {
        accountAddress: TxnBuilderTypes.AccountAddress.fromHex(CORE_CODE_ADDRESS),
        moduleName: "account",
        structName: "SignerCapabilityOfferProofChallengeV2",
        sequenceNumber: Number((await provider.aptosClient.getAccount(alice.address())).sequence_number),
        sourceAddress: TxnBuilderTypes.AccountAddress.fromHex(alice.address()),
        recipientAddress: TxnBuilderTypes.AccountAddress.fromHex(bob.address()),
      },
    );
    console.log({ hash, version, success, payload });
    const { data } = await provider.aptosClient.getAccountResource(alice.address(), "0x1::account::Account");
    const offerFor = (data as any).signer_capability_offer.for.vec[0];
    console.log({ signer_capability_offer: { for: offerFor }, bob: bob.address().toString() });
    assert(offerFor.toString() == bob.address().toString(), "Bob's address should be in the signer capability offer");
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

  // Note the hard-coded account scheme. You may need to accomodate MultiEd25519.
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
