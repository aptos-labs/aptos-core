import { AptosClient } from "./aptos_client";
import * as Gen from "./generated/index";
import { FAUCET_URL, NODE_URL } from "./util.test";
import { FaucetClient } from "./faucet_client";
import { AptosAccount } from "./aptos_account";
import { TxnBuilderTypes, TransactionBuilderMultiEd25519, BCS, TransactionBuilder } from "./transaction_builder";
import { TokenClient } from "./token_client";

const account = "0x1::account::Account";

const aptosCoin = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";

const coinTransferFunction = "0x1::coin::transfer";

test("gets genesis account", async () => {
  const client = new AptosClient(NODE_URL);
  const genesisAccount = await client.getAccount("0x1");
  expect(genesisAccount.authentication_key.length).toBe(66);
  expect(genesisAccount.sequence_number).not.toBeNull();
});

test("gets transactions", async () => {
  const client = new AptosClient(NODE_URL);
  const transactions = await client.getTransactions();
  expect(transactions.length).toBeGreaterThan(0);
});

test("gets genesis resources", async () => {
  const client = new AptosClient(NODE_URL);
  const resources = await client.getAccountResources("0x1");
  const accountResource = resources.find((r) => r.type == account);
  expect(accountResource).toBeDefined();
});

test("gets the Account resource", async () => {
  const client = new AptosClient(NODE_URL);
  const accountResource = await client.getAccountResource("0x1", account);
  expect(accountResource).toBeDefined();
});

test("gets ledger info", async () => {
  const client = new AptosClient(NODE_URL);
  const ledgerInfo = await client.getLedgerInfo();
  expect(ledgerInfo.chain_id).toBeGreaterThan(1);
  expect(parseInt(ledgerInfo.ledger_version, 10)).toBeGreaterThan(0);
});

test("gets account modules", async () => {
  const client = new AptosClient(NODE_URL);
  const modules = await client.getAccountModules("0x1");
  const module = modules.find((r) => r.abi!.name === "aptos_coin");
  expect(module!.abi!.address).toBe("0x1");
});

test("gets the AptosCoin module", async () => {
  const client = new AptosClient(NODE_URL);
  const module = await client.getAccountModule("0x1", "aptos_coin");
  expect(module!.abi!.address).toBe("0x1");
});

test(
  "submits bcs transaction",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

    const account1 = new AptosAccount();
    await faucetClient.fundAccount(account1.address(), 50000);
    let resources = await client.getAccountResources(account1.address());
    let accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("50000");

    const account2 = new AptosAccount();
    await faucetClient.fundAccount(account2.address(), 0);
    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("0");

    const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString("0x1::aptos_coin::AptosCoin"));

    const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        "0x1::coin",
        "transfer",
        [token],
        [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account2.address())), BCS.bcsSerializeUint64(717)],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      client.getAccount(account1.address()),
      client.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account1.address()),
      BigInt(sequnceNumber),
      scriptFunctionPayload,
      10000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const bcsTxn = AptosClient.generateBCSTransaction(account1, rawTxn);
    const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);

    await client.waitForTransaction(transactionRes.hash);

    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("717");
  },
  30 * 1000,
);

test(
  "submits multisig transaction",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

    const account1 = new AptosAccount();
    const account2 = new AptosAccount();
    const account3 = new AptosAccount();
    const multiSigPublicKey = new TxnBuilderTypes.MultiEd25519PublicKey(
      [
        new TxnBuilderTypes.Ed25519PublicKey(account1.signingKey.publicKey),
        new TxnBuilderTypes.Ed25519PublicKey(account2.signingKey.publicKey),
        new TxnBuilderTypes.Ed25519PublicKey(account3.signingKey.publicKey),
      ],
      2,
    );

    const authKey = TxnBuilderTypes.AuthenticationKey.fromMultiEd25519PublicKey(multiSigPublicKey);

    const mutisigAccountAddress = authKey.derivedAddress();
    await faucetClient.fundAccount(mutisigAccountAddress, 5000000);

    let resources = await client.getAccountResources(mutisigAccountAddress);
    let accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("5000000");

    const account4 = new AptosAccount();
    await faucetClient.fundAccount(account4.address(), 0);
    resources = await client.getAccountResources(account4.address());
    accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("0");

    const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString("0x1::aptos_coin::AptosCoin"));

    const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        "0x1::coin",
        "transfer",
        [token],
        [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account4.address())), BCS.bcsSerializeUint64(123)],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      client.getAccount(mutisigAccountAddress),
      client.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(mutisigAccountAddress),
      BigInt(sequnceNumber),
      scriptFunctionPayload,
      1000000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const txnBuilder = new TransactionBuilderMultiEd25519((signingMessage: TxnBuilderTypes.SigningMessage) => {
      const sigHexStr1 = account1.signBuffer(signingMessage);
      const sigHexStr3 = account3.signBuffer(signingMessage);
      const bitmap = TxnBuilderTypes.MultiEd25519Signature.createBitmap([0, 2]);

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
    accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("123");
  },
  30 * 1000,
);

test(
  "submits json transaction simulation",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

    const account1 = new AptosAccount();
    const account2 = new AptosAccount();
    const txns1 = await faucetClient.fundAccount(account1.address(), 1000000);
    const txns2 = await faucetClient.fundAccount(account2.address(), 1000000);
    const tx1 = await client.getTransactionByHash(txns1[1]);
    const tx2 = await client.getTransactionByHash(txns2[1]);
    expect(tx1.type).toBe("user_transaction");
    expect(tx2.type).toBe("user_transaction");
    const checkAptosCoin = async () => {
      const resources1 = await client.getAccountResources(account1.address());
      const resources2 = await client.getAccountResources(account2.address());
      const account1Resource = resources1.find((r) => r.type === aptosCoin);
      const account2Resource = resources2.find((r) => r.type === aptosCoin);
      expect((account1Resource!.data as { coin: { value: string } }).coin.value).toBe("1000000");
      expect((account2Resource!.data as { coin: { value: string } }).coin.value).toBe("1000000");
    };
    await checkAptosCoin();

    const payload: Gen.TransactionPayload = {
      type: "script_function_payload",
      function: coinTransferFunction,
      type_arguments: ["0x1::aptos_coin::AptosCoin"],
      arguments: [account2.address().hex(), "100000"],
    };
    const txnRequest = await client.generateTransaction(account1.address(), payload);
    const transactionRes = (await client.simulateTransaction(account1, txnRequest))[0];
    expect(parseInt(transactionRes.gas_used, 10) > 0);
    expect(transactionRes.success);
    const account2AptosCoin = transactionRes.changes.filter((change) => {
      if (change.type !== "write_resource") {
        return false;
      }
      const write = change as Gen.WriteResource;

      return (
        write.address === account2.address().hex() &&
        write.data.type === aptosCoin &&
        (write.data.data as { coin: { value: string } }).coin.value === "1100000"
      );
    });
    expect(account2AptosCoin).toHaveLength(1);
    await checkAptosCoin();
  },
  30 * 1000,
);

test(
  "submits bcs transaction simulation",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

    const account1 = new AptosAccount();
    const account2 = new AptosAccount();
    const txns1 = await faucetClient.fundAccount(account1.address(), 50000);
    const txns2 = await faucetClient.fundAccount(account2.address(), 10000);
    const tx1 = await client.getTransactionByHash(txns1[1]);
    const tx2 = await client.getTransactionByHash(txns2[1]);
    expect(tx1.type).toBe("user_transaction");
    expect(tx2.type).toBe("user_transaction");
    const checkAptosCoin = async () => {
      const resources1 = await client.getAccountResources(account1.address());
      const resources2 = await client.getAccountResources(account2.address());
      const account1Resource = resources1.find((r) => r.type === aptosCoin);
      const account2Resource = resources2.find((r) => r.type === aptosCoin);
      expect((account1Resource!.data as { coin: { value: string } }).coin.value).toBe("50000");
      expect((account2Resource!.data as { coin: { value: string } }).coin.value).toBe("10000");
    };
    await checkAptosCoin();

    const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString("0x1::aptos_coin::AptosCoin"));
    const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        "0x1::coin",
        "transfer",
        [token],
        [BCS.bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account2.address())), BCS.bcsSerializeUint64(1000)],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      client.getAccount(account1.address()),
      client.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      TxnBuilderTypes.AccountAddress.fromHex(account1.address()),
      BigInt(sequnceNumber),
      scriptFunctionPayload,
      10000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const bcsTxn = AptosClient.generateBCSSimulation(account1, rawTxn);
    const transactionRes = (await client.submitBCSSimulation(bcsTxn))[0];
    expect(parseInt(transactionRes.gas_used, 10) > 0);
    expect(transactionRes.success);
    const account2AptosCoin = transactionRes.changes.filter((change) => {
      if (change.type !== "write_resource") {
        return false;
      }
      const write = change as Gen.WriteResource;

      return (
        write.address === account2.address().toShortString() &&
        write.data.type === aptosCoin &&
        (write.data.data as { coin: { value: string } }).coin.value === "11000"
      );
    });
    expect(account2AptosCoin).toHaveLength(1);
    await checkAptosCoin();
  },
  30 * 1000,
);

// TODO: Delete this or fix it pending response in https://github.com/aptos-labs/aptos-core/pull/2164/.
test.skip(
  "submits multiagent transaction",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);
    const tokenClient = new TokenClient(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();
    const aliceAccountAddress = TxnBuilderTypes.AccountAddress.fromHex(alice.address());
    const bobAccountAddress = TxnBuilderTypes.AccountAddress.fromHex(bob.address());

    await faucetClient.fundAccount(alice.address(), 50000);

    let resources = await client.getAccountResources(alice.address());
    let accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("50000");

    await faucetClient.fundAccount(bob.address(), 60000);
    resources = await client.getAccountResources(bob.address());
    accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("60000");

    const collectionName = "AliceCollection";
    const tokenName = "Alice Token";

    // Create collection and token on Alice's account
    // eslint-disable-next-line quotes
    await tokenClient.createCollection(alice, collectionName, "Alice's simple collection", "https://aptos.dev");

    await tokenClient.createToken(
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
      ["int"],
    );

    let aliceBalance = await tokenClient.getTokenBalance(alice.address().hex(), collectionName, tokenName, "0");
    expect(aliceBalance.amount).toBe("1");

    const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        "0x1::token",
        "direct_transfer_script",
        [],
        [
          BCS.bcsToBytes(aliceAccountAddress),
          BCS.bcsSerializeStr(collectionName),
          BCS.bcsSerializeStr(tokenName),
          BCS.bcsSerializeUint64(1),
        ],
      ),
    );

    const [{ sequence_number: sequnceNumber }, chainId] = await Promise.all([
      client.getAccount(alice.address()),
      client.getChainId(),
    ]);

    const rawTxn = new TxnBuilderTypes.RawTransaction(
      aliceAccountAddress,
      BigInt(sequnceNumber),
      scriptFunctionPayload,
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const multiAgentTxn = new TxnBuilderTypes.MultiAgentRawTransaction(rawTxn, [bobAccountAddress]);

    const aliceSignature = new TxnBuilderTypes.Ed25519Signature(
      alice.signBuffer(TransactionBuilder.getSigningMessage(multiAgentTxn)).toUint8Array(),
    );

    const aliceAuthenticator = new TxnBuilderTypes.AccountAuthenticatorEd25519(
      new TxnBuilderTypes.Ed25519PublicKey(alice.signingKey.publicKey),
      aliceSignature,
    );

    const bobSignature = new TxnBuilderTypes.Ed25519Signature(
      bob.signBuffer(TransactionBuilder.getSigningMessage(multiAgentTxn)).toUint8Array(),
    );

    const bobAuthenticator = new TxnBuilderTypes.AccountAuthenticatorEd25519(
      new TxnBuilderTypes.Ed25519PublicKey(bob.signingKey.publicKey),
      bobSignature,
    );

    const multiAgentAuthenticator = new TxnBuilderTypes.TransactionAuthenticatorMultiAgent(
      aliceAuthenticator, // sender authenticator
      [bobAccountAddress], // secondary signer addresses
      [bobAuthenticator], // secondary signer authenticators
    );

    const bcsTxn = BCS.bcsToBytes(new TxnBuilderTypes.SignedTransaction(rawTxn, multiAgentAuthenticator));

    const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);

    await client.waitForTransaction(transactionRes.hash);

    const transaction = await client.getTransactionByHash(transactionRes.hash);
    expect((transaction as any)?.success).toBe(true);

    aliceBalance = await tokenClient.getTokenBalance(alice.address().hex(), collectionName, tokenName, "0");

    expect(aliceBalance.amount).toBe("0");

    const bobTokenStore = await client.getAccountResource(bob.address(), "0x1::token::TokenStore");

    const handle = (bobTokenStore.data as any).tokens?.handle;

    const getTokenTableItemRequest = {
      key_type: "0x1::token::TokenId",
      value_type: "0x1::token::Token",
      key: {
        creator: alice.address().hex(),
        collection: collectionName,
        name: tokenName,
      },
    };

    const bobTokenTableItem = await client.getTableItem(handle, getTokenTableItemRequest);
    expect(bobTokenTableItem?.value).toBe("1");
  },
  30 * 1000,
);
