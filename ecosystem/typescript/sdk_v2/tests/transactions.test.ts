import { hexToBytes } from "@noble/hashes/utils";
import { AptosAccount, NODE_URL, APTOS_FAUCET_URL } from "../src";
import { Aptos } from "../src/api/aptos";
import { createMultisigAccount } from "../src/transactions/generate_raw_transaction";
import {
  AptosTransactionPayload,
  FeePayerRawTransaction,
  MultiAgentRawTransaction,
  RawTransaction,
  TransactionArgumentU8,
  TransactionPayloadEntryFunction,
} from "../src/transactions/types";
import { TypeTagStruct, StructTag } from "../src/transactions/type_tag";

describe("generate raw transaction", () => {
  test("it gets a entry function type payload and output a raw transaction", async () => {
    const aptos = new Aptos({ network: NODE_URL, faucet: APTOS_FAUCET_URL });
    const alice = new AptosAccount();
    await aptos.transaction.fundAccount(alice.address(), 1000000);

    const bob = new AptosAccount();
    const payload: AptosTransactionPayload = {
      type: "entry_function",
      function: "0x1::aptos_account::transfer",
      type_arguments: [],
      arguments: [bob.address().hex(), 100000],
    };

    const response = await aptos.transaction.generate(alice.address(), payload);

    expect(response instanceof RawTransaction).toBeTruthy();
    expect((response as RawTransaction).sender.toHexString()).toEqual(alice.address().hex());
    expect(
      ((response as RawTransaction).payload as TransactionPayloadEntryFunction).value.module_name.address.toHexString(),
    ).toEqual("0x0000000000000000000000000000000000000000000000000000000000000001");
    expect(
      ((response as RawTransaction).payload as TransactionPayloadEntryFunction).value.module_name.name.value,
    ).toEqual("aptos_account");
    expect(((response as RawTransaction).payload as TransactionPayloadEntryFunction).value.function_name.value).toEqual(
      "transfer",
    );
  });

  test("it gets a script type payload and output a raw transaction", async () => {
    const aptos = new Aptos({ network: NODE_URL, faucet: APTOS_FAUCET_URL });
    const alice = new AptosAccount();
    await aptos.transaction.fundAccount(alice.address(), 1000000);

    const token = new TypeTagStruct(StructTag.fromString(`0x01::aptos_coin::AptosCoin`));

    const argU8 = new TransactionArgumentU8(2);

    const script = hexToBytes("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102");

    const payload: AptosTransactionPayload = {
      type: "script",
      bytecode: script,
      type_arguments: [token],
      arguments: [argU8],
    };

    const response = await aptos.transaction.generate(alice.address(), payload);

    expect(response instanceof RawTransaction).toBeTruthy();
    expect((response as RawTransaction).sender.toHexString()).toEqual(alice.address().hex());
  });
  test("it gets a multi agent type payload and output a raw transaction", async () => {
    const aptos = new Aptos({ network: NODE_URL, faucet: APTOS_FAUCET_URL });
    const alice = new AptosAccount();
    await aptos.transaction.fundAccount(alice.address(), 1000000);

    const bob = new AptosAccount();

    const payload: AptosTransactionPayload = {
      type: "multi_agent",
      function: "0x1::aptos_account::transfer",
      type_arguments: [],
      arguments: [bob.address().hex(), 100000],
      secondary_signer_addresses: [bob],
    };

    const response = await aptos.transaction.generate(alice.address(), payload);

    expect(response instanceof MultiAgentRawTransaction).toBeTruthy();
    expect((response as MultiAgentRawTransaction).raw_txn.sender.toHexString()).toEqual(alice.address().hex());
    expect((response as MultiAgentRawTransaction).secondary_signer_addresses[0].toHexString()).toEqual(
      bob.address().hex(),
    );
  });

  test("it gets a fee payer type payload and output a raw transaction", async () => {
    const aptos = new Aptos({ network: NODE_URL, faucet: APTOS_FAUCET_URL });
    const alice = new AptosAccount();
    await aptos.transaction.fundAccount(alice.address(), 1000000);

    const bob = new AptosAccount();

    const payload: AptosTransactionPayload = {
      type: "fee_payer",
      function: "0x1::aptos_account::transfer",
      type_arguments: [],
      arguments: [bob.address().hex(), 100000],
      secondary_signer_addresses: [bob],
      fee_payer: bob,
    };

    const response = await aptos.transaction.generate(alice.address(), payload);

    expect(response instanceof FeePayerRawTransaction).toBeTruthy();
    expect((response as FeePayerRawTransaction).raw_txn.sender.toHexString()).toEqual(alice.address().hex());
    expect((response as FeePayerRawTransaction).secondary_signer_addresses[0].toHexString()).toEqual(
      bob.address().hex(),
    );
    expect((response as FeePayerRawTransaction).fee_payer_address.toHexString()).toEqual(bob.address().hex());
  });
  test("it gets a multi sig payload and output a raw transaction", async () => {
    const aptos = new Aptos({ network: NODE_URL, faucet: APTOS_FAUCET_URL });
    const account1 = new AptosAccount();
    const account2 = new AptosAccount();
    const account3 = new AptosAccount();

    const multisigAccountAddress = createMultisigAccount([account1, account2, account3], 2);
    await aptos.transaction.fundAccount(multisigAccountAddress, 1000000);

    const bob = new AptosAccount();
    const payload: AptosTransactionPayload = {
      type: "multi_sig",
      function: "0x1::aptos_account::transfer",
      type_arguments: [],
      arguments: [bob.address().hex(), 100000],
    };
    const response = await aptos.transaction.generate(multisigAccountAddress, payload);

    expect(response instanceof RawTransaction).toBeTruthy();
    expect((response as RawTransaction).sender.toHexString()).toEqual(multisigAccountAddress);
  });
});
