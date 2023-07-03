/* eslint-disable no-console */

import dotenv from "dotenv";
dotenv.config();

import {
  AptosAccount,
  FaucetClient,
  BCS,
  TxnBuilderTypes,
  Types,
  HexString,
  FungibleAssetClient,
  Provider,
} from "aptos";
import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { FAUCET_URL, NODE_URL, fungibleStore } from "./common";
import assert from "assert";

const { AccountAddress, EntryFunction, MultiSig, MultiSigTransactionPayload, TransactionPayloadMultisig } =
  TxnBuilderTypes;

// Step 0: After publishing the example code with the example code under any `MODULE_ADDR` using CLI or any sdk.
const MODULE_ADDR = process.env.MODULE_ADDR;
const ASSET_SYMBOL = "MEME";

/**
 * This code example demonstrates how to use framework mulltisig account module to manage fungible asset with exmaple move code.
 */
(async () => {
  const client = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL /* not used */ });
  const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);
  const fa_client = new FungibleAssetClient(client);

  console.log(`node url: ${NODE_URL}`);
  console.log(`faucet url: ${FAUCET_URL}`);

  // Create and fund 3 accounts that will be the owners of the multisig account where owner1 would be the one that publish the move module later.
  // :!:>section_1
  const owner1 = new AptosAccount();
  const owner2 = new AptosAccount();
  const owner3 = new AptosAccount();
  await faucetClient.fundAccount(owner1.address(), 100_000_000);
  await faucetClient.fundAccount(owner2.address(), 100_000_000);
  await faucetClient.fundAccount(owner3.address(), 100_000_000);
  console.log(`owner1: ${owner1.address()}`);
  console.log(`owner2: ${owner2.address()}`);
  console.log(`owner3: ${owner3.address()}`);
  // <:!:section_1

  // Step 1: Setup a k-of-n (2-of-3 here) multisig account
  // ===========================================================================================
  // :!:>section_2
  // Find the next multisig account address.
  const payload: Types.ViewRequest = {
    function: "0x1::multisig_account::get_next_multisig_account_address",
    type_arguments: [],
    arguments: [owner1.address().hex()],
  };
  const multisigAddress = (await client.view(payload))[0] as string;

  const createMultisigManagedCoin = await client.generateTransaction(owner1.address(), {
    function: `${MODULE_ADDR}::multisig_managed_coin::initialize`,
    type_arguments: [],
    arguments: [
      [owner2.address().hex(), owner3.address().hex()],
      2,
      ["description"],
      [BCS.bcsSerializeStr("The multisig account for Meme")],
      0,
      "meme coin",
      ASSET_SYMBOL,
      8,
      "http://meme.xyz/favicon.ico",
      "http://meme.xyz",
      [true, true, true],
    ],
  });
  // <:!:section_2
  await client.generateSignSubmitWaitForTransaction(owner1, createMultisigManagedCoin.payload, { checkSuccess: true });
  assert((await getSignatureThreshold(client, multisigAddress)) == 2);
  assert((await getNumberOfOwners(client, multisigAddress)) == 3);

  // Fund the multisig account for gas.
  await faucetClient.fundAccount(multisigAddress, 100_000_000);

  // Step 2: Create a multisig transaction to mint coins to an account.
  // We'll be including the full payload to be stored on chain.
  // ===========================================================================================

  // Deterministically calculate the named metadata object address from the creator address and asset symbol.
  const metadata = getNamedObjectAddress(owner1, ASSET_SYMBOL);
  console.log(`metadata: ${metadata.toHexString()}`);

  // Create the mint transaction payload
  {
    // :!:>section_3
    const recipientsSerializer = new BCS.Serializer();
    BCS.serializeVector(
      [AccountAddress.fromHex(owner2.address()), AccountAddress.fromHex(owner3.address())],
      recipientsSerializer,
    );
    const mintTxPayload = new MultiSigTransactionPayload(
      EntryFunction.natural(
        `${MODULE_ADDR}::managed_fungible_asset`,
        "mint_to_primary_stores",
        [],
        [
          BCS.bcsToBytes(metadata),
          recipientsSerializer.getBytes(),
          BCS.serializeVectorWithFunc([1_000, 2_000], "serializeU64"),
        ],
      ),
    );
    const mintTxExecution = new TransactionPayloadMultisig(new MultiSig(AccountAddress.fromHex(multisigAddress)));

    // Create the mint multisig tx on chain.
    const mintTx = await client.generateTransaction(owner2.address(), {
      function: "0x1::multisig_account::create_transaction",
      type_arguments: [],
      arguments: [multisigAddress, BCS.bcsToBytes(mintTxPayload)],
    });
    await client.generateSignSubmitWaitForTransaction(owner2, mintTx.payload, { checkSuccess: true });

    // Owner 3 approves.
    await approve(client, owner3, multisigAddress, 1);

    // Owner 2 can now execute the transactions as it already has 2 approvals (from owners 2 and 3).
    await client.generateSignSubmitWaitForTransaction(owner2, mintTxExecution, { checkSuccess: true });
    // <:!:section_3
    // Check the primary store balance of owner2 and owner3.
    assert((await fa_client.getPrimaryBalance(owner2.address(), metadata.toHexString())) === BigInt(1_000));
    assert((await fa_client.getPrimaryBalance(owner3.address(), metadata.toHexString())) === BigInt(2_000));
  }

  // Step 3: Create another multisig transaction to freeze accounts but use payload hash instead.
  // ===========================================================================================
  // Create the reeze transaction payload
  {
    // :!:>section_4
    const freezeAccountsSerializer = new BCS.Serializer();
    BCS.serializeVector([AccountAddress.fromHex(owner1.address())], freezeAccountsSerializer);

    // Create freeze tx payload. The last paramter can be set to `false` to unfreeze.
    const freezeTxPayload = new MultiSigTransactionPayload(
      EntryFunction.natural(
        `${MODULE_ADDR}::managed_fungible_asset`,
        "set_primary_stores_frozen_status",
        [],
        [BCS.bcsToBytes(metadata), freezeAccountsSerializer.getBytes(), BCS.bcsSerializeBool(true)],
      ),
    );
    const multisigTxExecution = new TransactionPayloadMultisig(
      new MultiSig(AccountAddress.fromHex(multisigAddress), freezeTxPayload),
    );

    const transferTxPayloadHash = sha3Hash.create();
    transferTxPayloadHash.update(BCS.bcsToBytes(freezeTxPayload));
    const createMultisigTxWithHash = await client.generateTransaction(owner2.address(), {
      function: "0x1::multisig_account::create_transaction_with_hash",
      type_arguments: [],
      arguments: [multisigAddress, transferTxPayloadHash.digest()],
    });
    await client.generateSignSubmitWaitForTransaction(owner2, createMultisigTxWithHash.payload, { checkSuccess: true });
    await approve(client, owner1, multisigAddress, 2);

    const multisigTxExecution2 = new TransactionPayloadMultisig(
      new MultiSig(AccountAddress.fromHex(multisigAddress), freezeTxPayload),
    );
    await client.generateSignSubmitWaitForTransaction(owner2, multisigTxExecution2, { checkSuccess: true });
    // <:!:section_4
    let frozen = await client.view({
      function: "0x1::primary_fungible_store::is_frozen",
      type_arguments: ["0x1::fungible_asset::Metadata"],
      arguments: [owner1.address().hex(), metadata.toHexString()],
    });
    assert(frozen);
  }

  // Step 4: Create another multisig transaction to forcefully transfer fungible assets.
  // ===========================================================================================
  {
    // :!:>section_5
    const transferSendersSerializer = new BCS.Serializer();
    BCS.serializeVector([AccountAddress.fromHex(owner3.address())], transferSendersSerializer);
    const transferRecipentsSerializer = new BCS.Serializer();
    BCS.serializeVector([AccountAddress.fromHex(owner1.address())], transferRecipentsSerializer);
    const transferPayload = new MultiSigTransactionPayload(
      EntryFunction.natural(
        `${MODULE_ADDR}::managed_fungible_asset`,
        "transfer_between_primary_stores",
        [],
        [
          BCS.bcsToBytes(metadata),
          transferSendersSerializer.getBytes(),
          transferRecipentsSerializer.getBytes(),
          BCS.serializeVectorWithFunc([1_000], "serializeU64"),
        ],
      ),
    );
    const transferTx = await client.generateTransaction(owner2.address(), {
      function: "0x1::multisig_account::create_transaction",
      type_arguments: [],
      arguments: [multisigAddress, BCS.bcsToBytes(transferPayload)],
    });
    await client.generateSignSubmitWaitForTransaction(owner2, transferTx.payload, { checkSuccess: true });
    await approve(client, owner1, multisigAddress, 3);
    await client.generateSignSubmitWaitForTransaction(
      owner2,
      new TransactionPayloadMultisig(new MultiSig(AccountAddress.fromHex(multisigAddress))),
      { checkSuccess: true },
    );
    // <:!:section_5
    // Check the primary store balance of owner1 and owner3.
    assert((await fa_client.getPrimaryBalance(owner1.address(), metadata.toHexString())) === BigInt(1_000));
    assert((await fa_client.getPrimaryBalance(owner3.address(), metadata.toHexString())) === BigInt(1_000));
  }

  // Step 5: Create another multisig transaction to burn fungible assets.
  // ===========================================================================================
  {
    // :!:>section_6
    const burnAccountsSerializer = new BCS.Serializer();
    BCS.serializeVector(
      [
        AccountAddress.fromHex(owner1.address()),
        AccountAddress.fromHex(owner2.address()),
        AccountAddress.fromHex(owner3.address()),
      ],
      burnAccountsSerializer,
    );
    const burnPayload = new MultiSigTransactionPayload(
      EntryFunction.natural(
        `${MODULE_ADDR}::managed_fungible_asset`,
        "burn_from_primary_stores",
        [],
        [
          BCS.bcsToBytes(metadata),
          burnAccountsSerializer.getBytes(),
          BCS.serializeVectorWithFunc([1_000, 1_000, 1_000], "serializeU64"),
        ],
      ),
    );
    const burnTx = await client.generateTransaction(owner2.address(), {
      function: "0x1::multisig_account::create_transaction",
      type_arguments: [],
      arguments: [multisigAddress, BCS.bcsToBytes(burnPayload)],
    });
    await client.generateSignSubmitWaitForTransaction(owner2, burnTx.payload, { checkSuccess: true });
    await approve(client, owner1, multisigAddress, 4);
    await client.generateSignSubmitWaitForTransaction(
      owner2,
      new TransactionPayloadMultisig(new MultiSig(AccountAddress.fromHex(multisigAddress))),
      { checkSuccess: true },
    );
    // <:!:section_6
    // Check the primary store balance of owner1, owner2 and owner3.
    assert((await fa_client.getPrimaryBalance(owner1.address(), metadata.toHexString())) === BigInt(0));
    assert((await fa_client.getPrimaryBalance(owner2.address(), metadata.toHexString())) === BigInt(0));
    assert((await fa_client.getPrimaryBalance(owner3.address(), metadata.toHexString())) === BigInt(0));
    console.log("done.");
  }
})();

const approve = async (client: Provider, owner: AptosAccount, multisigAddress: string, transactionId: number) => {
  let approveTx = await client.generateTransaction(owner.address(), {
    function: "0x1::multisig_account::approve_transaction",
    type_arguments: [],
    arguments: [multisigAddress, transactionId],
  });
  await client.generateSignSubmitWaitForTransaction(owner, approveTx.payload, { checkSuccess: true });
};

const getNamedObjectAddress = (owner: AptosAccount, seed: string): TxnBuilderTypes.AccountAddress => {
  const hash = sha3Hash.create();
  hash.update(BCS.bcsToBytes(AccountAddress.fromHex(owner.address())));
  hash.update(seed);
  hash.update(new Uint8Array([0xfe]));
  return AccountAddress.fromHex(Buffer.from(hash.digest()).toString("hex"));
};

const getNumberOfOwners = async (client: Provider, multisigAddress: string): Promise<number> => {
  const multisigAccountResource = await client.getAccountResource(
    multisigAddress,
    "0x1::multisig_account::MultisigAccount",
  );
  return Number((multisigAccountResource.data as any).owners.length);
};

const getSignatureThreshold = async (client: Provider, multisigAddress: string): Promise<number> => {
  const multisigAccountResource = await client.getAccountResource(
    multisigAddress,
    "0x1::multisig_account::MultisigAccount",
  );
  return Number((multisigAccountResource.data as any).num_signatures_required);
};
