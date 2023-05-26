import { Http2Client } from ".";
import { AptosAccount } from "../account";
import { bcsToBytes, bcsSerializeUint64 } from "../bcs";
import { GetIndexerLedgerInfo } from "../indexer/generated/queries";
import { FaucetClient } from "../plugins";
import { AptosClient } from "../providers";
import { longTestTimeout } from "../tests/unit/test_helper.test";
import { TransactionBuilderRemoteABI, TxnBuilderTypes } from "../transaction_builder";

// test fetching from graphql
test(
  "fetching from graphql",
  async () => {
    const client = new Http2Client("https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql");
    const data = await client.post("/v1/graphql", JSON.stringify({ query: GetIndexerLedgerInfo }), {
      "content-type": "application/json",
    });
    console.log("data", data);
  },
  longTestTimeout,
);

// test submitting ABI transaction through fullnode
test(
  "submitting ABI transaction through fullnode",
  async () => {
    const aptosCoin = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
    const client = new AptosClient("https://fullnode.devnet.aptoslabs.com/v1");
    const faucetClient = new FaucetClient(
      "https://fullnode.devnet.aptoslabs.com/v1",
      "https://faucet.devnet.aptoslabs.com",
    );

    const account1 = new AptosAccount();
    await faucetClient.fundAccount(account1.address(), 100_000_000);
    let resources = await client.getAccountResources(account1.address());
    let accountResource = resources.find((r) => r.type === aptosCoin);
    expect((accountResource!.data as any).coin.value).toBe("100000000");

    const account2 = new AptosAccount();
    await faucetClient.fundAccount(account2.address(), 0);

    const builder = new TransactionBuilderRemoteABI(client, { sender: account1.address() });
    const rawTxn = await builder.build(
      "0x1::coin::transfer",
      ["0x1::aptos_coin::AptosCoin"],
      [account2.address(), 400],
    );

    const bcsTxn = AptosClient.generateBCSTransaction(account1, rawTxn);
    const http2client = new Http2Client("https://fullnode.devnet.aptoslabs.com");
    const transactionRes = await http2client.post("/v1/transactions", bcsTxn, {
      "content-type": "application/x.aptos.signed_transaction+bcs",
      "content-length": Buffer.byteLength(bcsTxn),
    });
    console.log("transactionRes", transactionRes);
  },
  longTestTimeout,
);

// test submitting bcs transaction through fullnode
test(
  "submitting bcs transaction through fullnode",
  async () => {
    const client = new AptosClient("https://fullnode.devnet.aptoslabs.com/v1");
    const faucetClient = new FaucetClient(
      "https://fullnode.devnet.aptoslabs.com/v1",
      "https://faucet.devnet.aptoslabs.com",
    );

    const account1 = new AptosAccount();
    await faucetClient.fundAccount(account1.address(), 100_000_000);

    const account2 = new AptosAccount();
    await faucetClient.fundAccount(account2.address(), 0);

    const token = new TxnBuilderTypes.TypeTagStruct(TxnBuilderTypes.StructTag.fromString("0x1::aptos_coin::AptosCoin"));
    const entryFunctionPayload = new TxnBuilderTypes.TransactionPayloadEntryFunction(
      TxnBuilderTypes.EntryFunction.natural(
        "0x1::coin",
        "transfer",
        [token],
        [bcsToBytes(TxnBuilderTypes.AccountAddress.fromHex(account2.address())), bcsSerializeUint64(1000)],
      ),
    );
    const rawTxn = await client.generateRawTransaction(account1.address(), entryFunctionPayload);
    const bcsTxn = AptosClient.generateBCSTransaction(account1, rawTxn);
    const http2client = new Http2Client("https://fullnode.devnet.aptoslabs.com");
    const transactionRes = await http2client.post("/v1/transactions", bcsTxn, {
      "content-type": "application/x.aptos.signed_transaction+bcs",
      "content-length": Buffer.byteLength(bcsTxn),
    });
    console.log("transactionRes", transactionRes);
  },
  longTestTimeout,
);

// test fetching from fullnode
test("fetching from full node", async () => {
  const client = new Http2Client("https://fullnode.devnet.aptoslabs.com");
  const data = await client.get("/v1", { "content-type": "application/json" });
  console.log(data);
});
