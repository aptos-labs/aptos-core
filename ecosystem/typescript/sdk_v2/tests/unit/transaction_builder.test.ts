import { AptosConfig } from "../../src";
import { Deserializer } from "../../src/bcs";
import { Account } from "../../src/core/account";
import { Ed25519PrivateKey } from "../../src/crypto/ed25519";
import { AccountAuthenticator, AccountAuthenticatorEd25519 } from "../../src/transactions/authenticator/account";
import {
  FeePayerRawTransaction,
  MultiAgentRawTransaction,
  RawTransaction,
  ScriptTransactionArgumentAddress,
  ScriptTransactionArgumentU64,
  TransactionPayloadEntryFunction,
  TransactionPayloadMultisig,
  TransactionPayloadScript,
} from "../../src/transactions/instances";
import {
  derieveTransactionType,
  generateRawTransaction,
  generateSignedTransaction,
  generateSignedTransactionForSimulation,
  generateTransaction,
  generateTransactionPayload,
  signTransaction,
} from "../../src/transactions/transaction_builder/transaction_builder";
import { SigningScheme } from "../../src/types";
import { Network } from "../../src/utils/api-endpoints";
import { SignedTransaction } from "../../src/transactions/instances/signedTransaction";
import { U64 } from "../../src/bcs/serializable/move-primitives";
import { MoveObject } from "../../src/bcs/serializable/move-structs";

describe("transaction builder", () => {
  describe("generate transaction payload", () => {
    test("it generates a script transaction payload", async () => {
      const payload = generateTransactionPayload({
        bytecode:
          "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
        type_arguments: [],
        arguments: [
          new U64(100),
          new U64(200),
          new MoveObject(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new MoveObject(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new U64(50),
        ],
      });
      expect(payload instanceof TransactionPayloadScript).toBeTruthy();
    });
    test("it generates a multi sig transaction payload", async () => {
      const payload = generateTransactionPayload({
        multisigAddress: Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress,
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [],
      });
      expect(payload instanceof TransactionPayloadMultisig).toBeTruthy();
    });
    test("it generates an entry function transaction payload", async () => {
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [],
      });
      expect(payload instanceof TransactionPayloadEntryFunction).toBeTruthy();
    });
  });
  describe("generate raw transaction", () => {
    test("it generates a raw transaction with script payload", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const payload = generateTransactionPayload({
        bytecode:
          "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
        type_arguments: [],
        arguments: [
          new ScriptTransactionArgumentU64(BigInt(100)),
          new ScriptTransactionArgumentU64(BigInt(200)),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentU64(BigInt(50)),
        ],
      });
      const rawTxn = await generateRawTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });
      expect(rawTxn instanceof RawTransaction).toBeTruthy();
      expect(rawTxn.payload instanceof TransactionPayloadScript).toBeTruthy();
    });

    test("it generates a raw transaction with a multi sig payload", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        multisigAddress: bob.accountAddress,
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const rawTxn = await generateRawTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });
      expect(rawTxn instanceof RawTransaction).toBeTruthy();
      expect(rawTxn.payload instanceof TransactionPayloadMultisig).toBeTruthy();
    });

    test("it generates a raw transaction with an entry function payload", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const rawTxn = await generateRawTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });
      expect(rawTxn instanceof RawTransaction).toBeTruthy();
      expect(rawTxn.payload instanceof TransactionPayloadEntryFunction).toBeTruthy();
    });
  });
  describe("generate transaction", () => {
    test("it returns a serialized raw transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const payload = generateTransactionPayload({
        bytecode:
          "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
        type_arguments: [],
        arguments: [
          new ScriptTransactionArgumentU64(BigInt(100)),
          new ScriptTransactionArgumentU64(BigInt(200)),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentU64(BigInt(50)),
        ],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });
      expect(transaction.rawTransaction instanceof Uint8Array).toBeTruthy();
      expect(transaction.secondarySignerAddresses).toBeUndefined();
      expect(transaction.feePayerAddress).toBeUndefined();
    });

    test("it returns a serialized raw transaction and secondary signers addresses", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const secondarySignerAddress = Account.generate({ scheme: SigningScheme.Ed25519 });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        secondarySignerAddresses: [secondarySignerAddress.accountAddress.toString()],
      });
      expect(transaction.rawTransaction instanceof Uint8Array).toBeTruthy();
      expect(transaction.secondarySignerAddresses).not.toBeUndefined();
      expect(transaction.secondarySignerAddresses?.length).toBe(1);
      expect(transaction.secondarySignerAddresses![0].data).toStrictEqual(
        secondarySignerAddress.accountAddress.toUint8Array(),
      );
      expect(transaction.feePayerAddress).toBeUndefined();
    });

    test("it returns a serialized raw transaction and a fee payer address", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const feePayer = Account.generate({ scheme: SigningScheme.Ed25519 });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        feePayerAddress: feePayer.accountAddress.toString(),
      });
      expect(transaction.rawTransaction instanceof Uint8Array).toBeTruthy();
      expect(transaction.secondarySignerAddresses?.length).toBe(0);
      expect(transaction.feePayerAddress).not.toBeUndefined();
      expect(transaction.feePayerAddress?.data).toStrictEqual(feePayer.accountAddress.toUint8Array());
    });

    test("it returns a serialized raw transaction, secondary signers addresses and a fee payer address", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const feePayer = Account.generate({ scheme: SigningScheme.Ed25519 });
      const secondarySignerAddress = Account.generate({ scheme: SigningScheme.Ed25519 });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        secondarySignerAddresses: [secondarySignerAddress.accountAddress.toString()],
        feePayerAddress: feePayer.accountAddress.toString(),
      });
      expect(transaction.rawTransaction instanceof Uint8Array).toBeTruthy();
      expect(transaction.secondarySignerAddresses).not.toBeUndefined();
      expect(transaction.secondarySignerAddresses?.length).toBe(1);
      expect(transaction.secondarySignerAddresses![0].data).toStrictEqual(
        secondarySignerAddress.accountAddress.toUint8Array(),
      );
      expect(transaction.feePayerAddress).not.toBeUndefined();
      expect(transaction.feePayerAddress?.data).toStrictEqual(feePayer.accountAddress.toUint8Array());
    });
  });
  describe("generateSignedTransactionForSimulation", () => {
    test("it generates a signed raw transaction for simulation", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const payload = generateTransactionPayload({
        bytecode:
          "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
        type_arguments: [],
        arguments: [
          new ScriptTransactionArgumentU64(BigInt(100)),
          new ScriptTransactionArgumentU64(BigInt(200)),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentU64(BigInt(50)),
        ],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });

      const bcsTransaction = await generateSignedTransactionForSimulation({
        transaction,
        signerPublicKey: alice.publicKey,
      });
      expect(bcsTransaction instanceof Uint8Array).toBeTruthy();
      const deserializer = new Deserializer(bcsTransaction);
      const signedTransaction = SignedTransaction.deserialize(deserializer);
      expect(signedTransaction instanceof SignedTransaction).toBeTruthy();
    });
  });
  describe("sign", () => {
    test("it signs a raw transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const payload = generateTransactionPayload({
        bytecode:
          "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
        type_arguments: [],
        arguments: [
          new ScriptTransactionArgumentU64(BigInt(100)),
          new ScriptTransactionArgumentU64(BigInt(200)),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentU64(BigInt(50)),
        ],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });
      const accountAuthenticator = signTransaction({ signer: alice, transaction });
      expect(accountAuthenticator instanceof AccountAuthenticator).toBeTruthy();
      const deserializer = new Deserializer(accountAuthenticator.bcsToBytes());
      const authenticator = AccountAuthenticator.deserialize(deserializer);
      expect(authenticator instanceof AccountAuthenticatorEd25519).toBeTruthy();
    });

    test("it signs a fee payer transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        multisigAddress: bob.accountAddress,
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        feePayerAddress: Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress.toString(),
      });
      const accountAuthenticator = signTransaction({ signer: alice, transaction });
      expect(accountAuthenticator instanceof AccountAuthenticator).toBeTruthy();
      const deserializer = new Deserializer(accountAuthenticator.bcsToBytes());
      const authenticator = AccountAuthenticator.deserialize(deserializer);
      expect(authenticator instanceof AccountAuthenticatorEd25519).toBeTruthy();
    });

    test("it signs a multi agent transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        bytecode:
          "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
        type_arguments: [],
        arguments: [
          new ScriptTransactionArgumentU64(BigInt(100)),
          new ScriptTransactionArgumentU64(BigInt(200)),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentU64(BigInt(50)),
        ],
      });
      const rawTxn = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        secondarySignerAddresses: [bob.accountAddress.toString()],
      });
      const accountAuthenticator = signTransaction({ signer: alice, transaction: rawTxn });
      expect(accountAuthenticator instanceof AccountAuthenticator).toBeTruthy();
      const deserializer = new Deserializer(accountAuthenticator.bcsToBytes());
      const authenticator = AccountAuthenticator.deserialize(deserializer);
      expect(authenticator instanceof AccountAuthenticatorEd25519).toBeTruthy();
    });
  });
  describe("generateSignedTransaction", () => {
    test("it generates a single signer signed transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const payload = generateTransactionPayload({
        bytecode:
          "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
        type_arguments: [],
        arguments: [
          new ScriptTransactionArgumentU64(BigInt(100)),
          new ScriptTransactionArgumentU64(BigInt(200)),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentAddress(Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress),
          new ScriptTransactionArgumentU64(BigInt(50)),
        ],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });
      const authenticator = signTransaction({ signer: alice, transaction });
      const bcsTransaction = await generateSignedTransaction({
        transaction,
        senderAuthenticator: authenticator,
      });
      expect(bcsTransaction instanceof Uint8Array).toBeTruthy();
      const deserializer = new Deserializer(bcsTransaction);
      const signedTransaction = SignedTransaction.deserialize(deserializer);
      expect(signedTransaction instanceof SignedTransaction).toBeTruthy();
    });

    test("it generates a multi agent signed transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        secondarySignerAddresses: [bob.accountAddress.toString()],
      });
      const authenticator = signTransaction({ signer: alice, transaction });
      const secondaryAuthenticator = signTransaction({ signer: bob, transaction });
      const bcsTransaction = await generateSignedTransaction({
        transaction,
        senderAuthenticator: authenticator,
        secondarySignerAuthenticators: { additionalSignersAuthenticators: [secondaryAuthenticator] },
      });
      expect(bcsTransaction instanceof Uint8Array).toBeTruthy();
      const deserializer = new Deserializer(bcsTransaction);
      const signedTransaction = SignedTransaction.deserialize(deserializer);
      expect(signedTransaction instanceof SignedTransaction).toBeTruthy();
    });

    test("it generates a fee payer signed transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        feePayerAddress: bob.accountAddress.toString(),
      });
      const authenticator = signTransaction({ signer: alice, transaction });
      const feePayerAuthenticator = signTransaction({ signer: bob, transaction });
      const bcsTransaction = await generateSignedTransaction({
        transaction,
        senderAuthenticator: authenticator,
        secondarySignerAuthenticators: { feePayerAuthenticator: feePayerAuthenticator },
      });
      expect(bcsTransaction instanceof Uint8Array).toBeTruthy();
      const deserializer = new Deserializer(bcsTransaction);
      const signedTransaction = SignedTransaction.deserialize(deserializer);
      expect(signedTransaction instanceof SignedTransaction).toBeTruthy();
    });
  });
  describe("derieveTransactionType", () => {
    test("it derieves the transaction type as a RawTransaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
      });
      const transactionType = derieveTransactionType(transaction);
      expect(transactionType instanceof RawTransaction).toBeTruthy();
    });

    test("it derieves the transaction type as a FeePayerRawTransaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        feePayerAddress: Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress.toString(),
      });

      const transactionType = derieveTransactionType(transaction);
      expect(transactionType instanceof FeePayerRawTransaction).toBeTruthy();
    });

    test("it derieves the transaction type as a MultiAgentRawTransaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const payload = generateTransactionPayload({
        function: "0x1::aptos_account::transfer",
        type_arguments: [],
        arguments: [new MoveObject(bob.accountAddress), new U64(1)],
      });
      const transaction = await generateTransaction({
        aptosConfig: config,
        sender: alice.accountAddress.toString(),
        payload,
        secondarySignerAddresses: [Account.generate({ scheme: SigningScheme.Ed25519 }).accountAddress.toString()],
      });
      const transactionType = derieveTransactionType(transaction);
      expect(transactionType instanceof MultiAgentRawTransaction).toBeTruthy();
    });
  });
});
