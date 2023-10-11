import { AptosConfig, Network, Aptos, Deserializer } from "../../../src";
import { U64 } from "../../../src/bcs/serializable/move-primitives";
import { MoveObject } from "../../../src/bcs/serializable/move-structs";
import { Account } from "../../../src/core/account";
import { Ed25519PrivateKey } from "../../../src/crypto/ed25519";
import { AccountAuthenticator, AccountAuthenticatorEd25519 } from "../../../src/transactions/authenticator/account";
import {
  RawTransaction,
  ScriptTransactionArgumentAddress,
  ScriptTransactionArgumentU64,
} from "../../../src/transactions/instances";
import {
  TransactionPayloadEntryFunction,
  TransactionPayloadMultisig,
  TransactionPayloadScript,
} from "../../../src/transactions/instances/transactionPayload";
import { SigningScheme } from "../../../src/types";

describe("transaction submission", () => {
  describe("generateTransaction", () => {
    test("it generates a script transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        data: {
          bytecode: "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
          type_arguments: [],
          arguments: [],
        },
      });
      expect(rawTxn.rawTransaction instanceof Uint8Array).toBeTruthy();
      const deserializer = new Deserializer(rawTxn.rawTransaction);
      const desirializedTransaction = RawTransaction.deserialize(deserializer);
      expect(desirializedTransaction instanceof RawTransaction).toBeTruthy();
      expect(desirializedTransaction.payload instanceof TransactionPayloadScript).toBeTruthy();
    });

    test("it generates a multi sig transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        data: {
          multisigAddress: bob.accountAddress,
          function: "0x1::aptos_account::transfer",
          type_arguments: [],
          arguments: [new MoveObject(bob.accountAddress), new U64(1)],
        },
      });
      expect(rawTxn.rawTransaction instanceof Uint8Array).toBeTruthy();
      const deserializer = new Deserializer(rawTxn.rawTransaction);
      const desirializedTransaction = RawTransaction.deserialize(deserializer);
      expect(desirializedTransaction instanceof RawTransaction).toBeTruthy();
      expect(desirializedTransaction.payload instanceof TransactionPayloadMultisig).toBeTruthy();
    });

    test("it generates an entry function transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        data: {
          function: "0x1::aptos_account::transfer",
          type_arguments: [],
          arguments: [new MoveObject(bob.accountAddress), new U64(1)],
        },
      });
      expect(rawTxn.rawTransaction instanceof Uint8Array).toBeTruthy();
      const deserializer = new Deserializer(rawTxn.rawTransaction);
      const desirializedTransaction = RawTransaction.deserialize(deserializer);
      expect(desirializedTransaction instanceof RawTransaction).toBeTruthy();
      expect(desirializedTransaction.payload instanceof TransactionPayloadEntryFunction).toBeTruthy();
    });
  });
  describe("simultaeTransaction", () => {
    test("it simulates a multi agent script transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0xc1e06f150f7fa79c1ca0ad0f8d0b26d83abaf1bed0fc31de24b500d81a2ff924",
        }),
      });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        secondarySignerAddresses: [bob.accountAddress.toString()],
        data: {
          bytecode:
            "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
          type_arguments: [],
          arguments: [
            new ScriptTransactionArgumentU64(BigInt(100)),
            new ScriptTransactionArgumentU64(BigInt(200)),
            new ScriptTransactionArgumentAddress(bob.accountAddress),
            new ScriptTransactionArgumentAddress(alice.accountAddress),
            new ScriptTransactionArgumentU64(BigInt(50)),
          ],
        },
      });

      const [response] = await aptos.simulateTransaction({
        signerPublicKey: alice.publicKey,
        transaction: rawTxn,
        secondarySignersPublicKeys: [bob.publicKey],
      });
      expect(response.success).toBeTruthy();
    });
  });
  describe("signTransaction", () => {
    test("it signs a script transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        data: {
          bytecode: "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
          type_arguments: [],
          arguments: [],
        },
      });
      const accountAuthenticator = aptos.signTransaction({ signer: alice, transaction: rawTxn });
      expect(accountAuthenticator instanceof AccountAuthenticator).toBeTruthy();
      const deserializer = new Deserializer(accountAuthenticator.bcsToBytes());
      const authenticator = AccountAuthenticator.deserialize(deserializer);
      expect(authenticator instanceof AccountAuthenticatorEd25519).toBeTruthy();
    });

    test("it signs a multi sig transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        data: {
          multisigAddress: bob.accountAddress,
          function: "0x1::aptos_account::transfer",
          type_arguments: [],
          arguments: [new MoveObject(bob.accountAddress), new U64(1)],
        },
      });
      const accountAuthenticator = aptos.signTransaction({ signer: alice, transaction: rawTxn });
      expect(accountAuthenticator instanceof AccountAuthenticator).toBeTruthy();
      const deserializer = new Deserializer(accountAuthenticator.bcsToBytes());
      const authenticator = AccountAuthenticator.deserialize(deserializer);
      expect(authenticator instanceof AccountAuthenticatorEd25519).toBeTruthy();
    });

    test("it signs an entry function transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        data: {
          function: "0x1::aptos_account::transfer",
          type_arguments: [],
          arguments: [new MoveObject(bob.accountAddress), new U64(1)],
        },
      });
      const accountAuthenticator = aptos.signTransaction({ signer: alice, transaction: rawTxn });
      expect(accountAuthenticator instanceof AccountAuthenticator).toBeTruthy();
      const deserializer = new Deserializer(accountAuthenticator.bcsToBytes());
      const authenticator = AccountAuthenticator.deserialize(deserializer);
      expect(authenticator instanceof AccountAuthenticatorEd25519).toBeTruthy();
    });
  });
  describe("submitTransaction", () => {
    test("it submits a script transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0xc1e06f150f7fa79c1ca0ad0f8d0b26d83abaf1bed0fc31de24b500d81a2ff924",
        }),
      });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        secondarySignerAddresses: [bob.accountAddress.toString()],
        data: {
          bytecode:
            "a11ceb0b060000000701000402040a030e18042608052e4307713e08af01200000000101020401000100030800010403040100010505060100010607040100010708060100000201020202030207060c060c0303050503030b000108010b000108010b0001080101080102060c03010b0001090002070b000109000b000109000002070b000109000302050b000109000a6170746f735f636f696e04636f696e04436f696e094170746f73436f696e087769746864726177056d657267650765787472616374076465706f73697400000000000000000000000000000000000000000000000000000000000000010000011a0b000a0238000c070b010a0338000c080d070b0838010d070b020b03160b061738020c090b040b0738030b050b09380302",
          type_arguments: [],
          arguments: [
            new ScriptTransactionArgumentU64(BigInt(100)),
            new ScriptTransactionArgumentU64(BigInt(200)),
            new ScriptTransactionArgumentAddress(bob.accountAddress),
            new ScriptTransactionArgumentAddress(alice.accountAddress),
            new ScriptTransactionArgumentU64(BigInt(50)),
          ],
        },
      });
      const authenticator = aptos.signTransaction({ signer: alice, transaction: rawTxn });
      const bobauthenticator = aptos.signTransaction({ signer: bob, transaction: rawTxn });
      const response = await aptos.submitTransaction({
        transaction: rawTxn,
        senderAuthenticator: authenticator,
        secondarySignerAuthenticators: { additionalSignersAuthenticators: [bobauthenticator] },
      });
      expect(response).toHaveProperty("hash");
    });

    // Currently this test fails because we don't wait for the previous transaction to be executed
    // and we end up with transaction is already in mempool. This is because we are still missing
    // waitForTransaction query and/or the fund query (so we can create an account and then fund it
    // to create it on chain) - Anyhow, I tested each test individually and it works.
    // The whole test flow should work once we have the option to wait for transaction and/or fund
    // an account to create it on chain
    test("it submits an entry function transaction", async () => {
      const config = new AptosConfig({ network: Network.DEVNET });
      const aptos = new Aptos(config);
      const alice = Account.fromPrivateKey({
        privateKey: new Ed25519PrivateKey({
          hexInput: "0x5aba8dab1c523be32bd4dafe2cc612f7f8050ce42a3322b60216ef67dc97768c",
        }),
      });
      const bob = Account.generate({ scheme: SigningScheme.Ed25519 });
      const rawTxn = await aptos.generateTransaction({
        sender: alice.accountAddress.toString(),
        data: {
          function: "0x1::aptos_account::transfer",
          type_arguments: [],
          arguments: [new MoveObject(bob.accountAddress), new U64(1)],
        },
      });
      const authenticator = aptos.signTransaction({ signer: alice, transaction: rawTxn });
      const response = await aptos.submitTransaction({ transaction: rawTxn, senderAuthenticator: authenticator });
      expect(response).toHaveProperty("hash");
    });
  });
});
