/* eslint-disable no-console */
import { AptosClient, AptosAccount, FaucetClient, BCS, TxnBuilderTypes, TransactionBuilderMultiEd25519 } from "aptos";
import assert from "assert";

const NODE_URL = process.env.APTOS_NODE_URL || "https://fullnode.devnet.aptoslabs.com";
const FAUCET_URL = process.env.APTOS_FAUCET_URL || "https://faucet.devnet.aptoslabs.com";

type SigningMessage = TxnBuilderTypes.SigningMessage;

/**
 * This code example demonstrates the process of moving test coins from one multisig
 * account to a single signature account.
 */
(async () => {
  const client = new AptosClient(NODE_URL);
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

  // Genereate 3 key pairs and account instances
  const account1 = new AptosAccount();
  const account2 = new AptosAccount();
  const account3 = new AptosAccount();

  // Create a 2 out of 3 MultiEd25519PublicKey. '2 out of 3' means for a multisig transaction
  // to be executed, at least 2 accounts must have signed the transaction.
  // See https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.MultiEd25519PublicKey.html#constructor
  const multiSigPublicKey = new TxnBuilderTypes.MultiEd25519PublicKey(
    [
      new TxnBuilderTypes.Ed25519PublicKey(account1.signingKey.publicKey),
      new TxnBuilderTypes.Ed25519PublicKey(account2.signingKey.publicKey),
      new TxnBuilderTypes.Ed25519PublicKey(account3.signingKey.publicKey),
    ],
    // Threshold
    2,
  );

  // Each Aptos account stores an auth key. Initial account address can be derived from auth key.
  // See https://aptos.dev/basics/basics-accounts for more details.
  const authKey = TxnBuilderTypes.AuthenticationKey.fromMultiEd25519PublicKey(multiSigPublicKey);

  // Derive the multisig account address and fund the address with 5000 AptosCoin.
  const mutisigAccountAddress = authKey.derivedAddress();
  await faucetClient.fundAccount(mutisigAccountAddress, 5000);

  let resources = await client.getAccountResources(mutisigAccountAddress);
  let accountResource = resources.find((r) => r.type === "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>");
  let balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 5000);
  console.log(`multisig account coins: ${balance}. Should be 5000!`);

  const account4 = new AptosAccount();
  // Creates a receiver account and fund the account with 0 AptosCoin
  await faucetClient.fundAccount(account4.address(), 0);
  resources = await client.getAccountResources(account4.address());
  accountResource = resources.find((r) => r.type === "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>");
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 0);
  console.log(`multisig account coins: ${balance}. Should be 0!`);

  const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString("0x1::aptos_coin::AptosCoin"));

  // TS SDK support 3 types of transaction payloads: `ScriptFunction`, `Script` and `Module`.
  // See https://aptos-labs.github.io/ts-sdk-doc/ for the details.
  const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
    TxnBuilderTypes.ScriptFunction.natural(
      // Fully qualified module name, `AccountAddress::ModuleName`
      "0x1::coin",
      // Module function
      "transfer",
      // The coin type to transfer
      [token],
      // Arguments for function `transfer`: receiver account address and amount to transfer
      [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account4.address())), BCS.bcsSerializeUint64(123)],
    ),
  );

  const [{ sequence_number: sequenceNumber }, chainId] = await Promise.all([
    client.getAccount(mutisigAccountAddress),
    client.getChainId(),
  ]);

  // See class definiton here
  // https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.RawTransaction.html#constructor.
  const rawTxn = new TxnBuilderTypes.RawTransaction(
    // Transaction sender account address
    TxnBuilderTypes.AccountAddress.fromHex(mutisigAccountAddress),
    BigInt(sequenceNumber),
    scriptFunctionPayload,
    // Max gas unit to spend
    1000n,
    // Gas price per unit
    1n,
    // Expiration timestamp. Transaction is discarded if it is not executed within 10 seconds from now.
    BigInt(Math.floor(Date.now() / 1000) + 10),
    new TxnBuilderTypes.ChainId(chainId),
  );

  // account1 and account3 sign the transaction
  const txnBuilder = new TransactionBuilderMultiEd25519((signingMessage: SigningMessage) => {
    const sigHexStr1 = account1.signBuffer(signingMessage);
    const sigHexStr3 = account3.signBuffer(signingMessage);

    // Bitmap masks which public key has signed transaction.
    // See https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.MultiEd25519Signature.html#createBitmap
    const bitmap = TxnBuilderTypes.MultiEd25519Signature.createBitmap([0, 2]);

    // See https://aptos-labs.github.io/ts-sdk-doc/classes/TxnBuilderTypes.MultiEd25519Signature.html#constructor
    const muliEd25519Sig = new TxnBuilderTypes.MultiEd25519Signature(
      [
        new TxnBuilderTypes.Ed25519Signature(sigHexStr1.toUint8Array()),
        new TxnBuilderTypes.Ed25519Signature(sigHexStr3.toUint8Array()),
      ],
      bitmap,
    );

    return muliEd25519Sig;
  }, multiSigPublicKey);

  const bcsTxn = txnBuilder.sign(rawTxn);
  const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);

  await client.waitForTransaction(transactionRes.hash);

  resources = await client.getAccountResources(account4.address());
  accountResource = resources.find((r) => r.type === "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>");
  balance = parseInt((accountResource?.data as any).coin.value);
  assert(balance === 123);
  console.log(`multisig account coins: ${balance}. Should be 123!`);
})();
