/* eslint-disable no-bitwise */
import { AxiosResponse } from 'axios';
import { AptosClient, raiseForStatus } from './aptos_client';
import { AnyObject } from './util';

import { FAUCET_URL, NODE_URL } from './util.test';
import { FaucetClient } from './faucet_client';
import { AptosAccount } from './aptos_account';
import { TxnBuilderTypes, TransactionBuilderMultiEd25519, BCS } from './transaction_builder';
import { TransactionPayload, WriteResource } from './api/data-contracts';

test('gets genesis account', async () => {
  const client = new AptosClient(NODE_URL);
  const account = await client.getAccount('0x1');
  expect(account.authentication_key.length).toBe(66);
  expect(account.sequence_number).not.toBeNull();
});

test('gets transactions', async () => {
  const client = new AptosClient(NODE_URL);
  const transactions = await client.getTransactions();
  expect(transactions.length).toBeGreaterThan(0);
});

test('gets genesis resources', async () => {
  const client = new AptosClient(NODE_URL);
  const resources = await client.getAccountResources('0x1');
  const accountResource = resources.find((r) => r.type === '0x1::Account::Account');
  expect((accountResource.data as AnyObject).self_address).toBe('0x1');
});

test('gets the Account resource', async () => {
  const client = new AptosClient(NODE_URL);
  const accountResource = await client.getAccountResource('0x1', '0x1::Account::Account');
  expect((accountResource.data as AnyObject).self_address).toBe('0x1');
});

test('gets ledger info', async () => {
  const client = new AptosClient(NODE_URL);
  const ledgerInfo = await client.getLedgerInfo();
  expect(ledgerInfo.chain_id).toBeGreaterThan(1);
  expect(parseInt(ledgerInfo.ledger_version, 10)).toBeGreaterThan(0);
});

test('gets account modules', async () => {
  const client = new AptosClient(NODE_URL);
  const modules = await client.getAccountModules('0x1');
  const module = modules.find((r) => r.abi.name === 'TestCoin');
  expect(module.abi.address).toBe('0x1');
});

test('gets the TestCoin module', async () => {
  const client = new AptosClient(NODE_URL);
  const module = await client.getAccountModule('0x1', 'TestCoin');
  expect(module.abi.address).toBe('0x1');
});

test('test raiseForStatus', async () => {
  const testData = { hello: 'wow' };
  const fakeResponse: AxiosResponse = {
    status: 200,
    statusText: 'Status Text',
    data: 'some string',
    request: {
      host: 'host',
      path: '/path',
    },
  } as AxiosResponse;

  // Shouldn't throw
  raiseForStatus(200, fakeResponse, testData);
  raiseForStatus(200, fakeResponse);

  // an error, oh no!
  fakeResponse.status = 500;
  expect(() => raiseForStatus(200, fakeResponse, testData)).toThrow(
    'Status Text - "some string" @ host/path : {"hello":"wow"}',
  );

  expect(() => raiseForStatus(200, fakeResponse)).toThrow('Status Text - "some string" @ host/path');

  // Just a wild test to make sure it doesn't break: request is `any`!
  delete fakeResponse.request;
  expect(() => raiseForStatus(200, fakeResponse, testData)).toThrow('Status Text - "some string" : {"hello":"wow"}');

  expect(() => raiseForStatus(200, fakeResponse)).toThrow('Status Text - "some string"');
});

test(
  'submits bcs transaction',
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL, null);

    const account1 = new AptosAccount();
    await faucetClient.fundAccount(account1.address(), 5000);
    let resources = await client.getAccountResources(account1.address());
    let accountResource = resources.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
    expect((accountResource.data as any).coin.value).toBe('5000');

    const account2 = new AptosAccount();
    await faucetClient.fundAccount(account2.address(), 0);
    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
    expect((accountResource.data as any).coin.value).toBe('0');

    const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString('0x1::TestCoin::TestCoin'));

    const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        '0x1::Coin',
        'transfer',
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
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const bcsTxn = AptosClient.generateBCSTransaction(account1, rawTxn);
    const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);

    await client.waitForTransaction(transactionRes.hash);

    resources = await client.getAccountResources(account2.address());
    accountResource = resources.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
    expect((accountResource.data as any).coin.value).toBe('717');
  },
  30 * 1000,
);

test(
  'submits multisig transaction',
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL, null);

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
    await faucetClient.fundAccount(mutisigAccountAddress, 5000);

    let resources = await client.getAccountResources(mutisigAccountAddress);
    let accountResource = resources.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
    expect((accountResource.data as any).coin.value).toBe('5000');

    const account4 = new AptosAccount();
    await faucetClient.fundAccount(account4.address(), 0);
    resources = await client.getAccountResources(account4.address());
    accountResource = resources.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
    expect((accountResource.data as any).coin.value).toBe('0');

    const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString('0x1::TestCoin::TestCoin'));

    const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natural(
        '0x1::Coin',
        'transfer',
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
      1000n,
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
    accountResource = resources.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
    expect((accountResource.data as any).coin.value).toBe('123');
  },
  30 * 1000,
);

test(
  'submits json transaction simulation',
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

    const account1 = new AptosAccount();
    const account2 = new AptosAccount();
    const txns1 = await faucetClient.fundAccount(account1.address(), 5000);
    const txns2 = await faucetClient.fundAccount(account2.address(), 1000);
    const tx1 = await client.getTransaction(txns1[1]);
    const tx2 = await client.getTransaction(txns2[1]);
    expect(tx1.type).toBe('user_transaction');
    expect(tx2.type).toBe('user_transaction');
    const checkTestCoin = async () => {
      let resources1 = await client.getAccountResources(account1.address());
      let resources2 = await client.getAccountResources(account2.address());
      let account1Resource = resources1.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
      let account2Resource = resources2.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
      expect((account1Resource.data as { coin: { value: string } }).coin.value).toBe('5000');
      expect((account2Resource.data as { coin: { value: string } }).coin.value).toBe('1000');
    };
    await checkTestCoin();

    const payload: TransactionPayload = {
      type: 'script_function_payload',
      function: '0x1::Coin::transfer',
      type_arguments: ['0x1::TestCoin::TestCoin'],
      arguments: [account2.address().hex(), '1000'],
    };
    const txnRequest = await client.generateTransaction(account1.address(), payload);
    const transactionRes = await client.simulateTransaction(account1, txnRequest);
    expect(parseInt(transactionRes.gas_used) > 0);
    expect(transactionRes.success);
    const account2TestCoin = transactionRes.changes.filter((change) => {
      if (change.type !== 'write_resource') {
        return false;
      }
      const write = change as WriteResource;

      return (
        write.address === account2.address().hex() &&
        write.data.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>' &&
        (write.data.data as { coin: { value: string } }).coin.value === '2000'
      );
    });
    expect(account2TestCoin).toHaveLength(1);
    await checkTestCoin();
  },
  30 * 1000,
);

test(
  'submits bcs transaction simulation',
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);

    const account1 = new AptosAccount();
    const account2 = new AptosAccount();
    const txns1 = await faucetClient.fundAccount(account1.address(), 5000);
    const txns2 = await faucetClient.fundAccount(account2.address(), 1000);
    const tx1 = await client.getTransaction(txns1[1]);
    const tx2 = await client.getTransaction(txns2[1]);
    expect(tx1.type).toBe('user_transaction');
    expect(tx2.type).toBe('user_transaction');
    const checkTestCoin = async () => {
      let resources1 = await client.getAccountResources(account1.address());
      let resources2 = await client.getAccountResources(account2.address());
      let account1Resource = resources1.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
      let account2Resource = resources2.find((r) => r.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>');
      expect((account1Resource.data as { coin: { value: string } }).coin.value).toBe('5000');
      expect((account2Resource.data as { coin: { value: string } }).coin.value).toBe('1000');
    };
    await checkTestCoin();

    const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString('0x1::TestCoin::TestCoin'));
    const scriptFunctionPayload = new TxnBuilderTypes.TransactionPayloadScriptFunction(
      TxnBuilderTypes.ScriptFunction.natual(
        '0x1::Coin',
        'transfer',
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
      1000n,
      1n,
      BigInt(Math.floor(Date.now() / 1000) + 10),
      new TxnBuilderTypes.ChainId(chainId),
    );

    const bcsTxn = AptosClient.generateBCSSimulation(account1, rawTxn);
    const transactionRes = await client.submitBCSSimulation(bcsTxn);
    expect(parseInt(transactionRes.gas_used) > 0);
    expect(transactionRes.success);
    const account2TestCoin = transactionRes.changes.filter((change) => {
      if (change.type !== 'write_resource') {
        return false;
      }
      const write = change as WriteResource;

      return (
        write.address === account2.address().hex() &&
        write.data.type === '0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>' &&
        (write.data.data as { coin: { value: string } }).coin.value === '2000'
      );
    });
    expect(account2TestCoin).toHaveLength(1);
    await checkTestCoin();
  },
  30 * 1000,
);
