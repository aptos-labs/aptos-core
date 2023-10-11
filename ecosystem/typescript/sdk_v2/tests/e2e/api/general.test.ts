import { AptosConfig, Aptos } from "../../../src";
import { GraphqlQuery, ViewRequest } from "../../../src/types";
import { Network } from "../../../src/utils/api-endpoints";

describe("general api", () => {
  test("it fetches ledger info", async () => {
    const config = new AptosConfig({ network: Network.LOCAL });
    const aptos = new Aptos(config);
    const ledgerInfo = await aptos.getLedgerInfo();
    expect(ledgerInfo.chain_id).toBe(4);
  });

  test("it fetches chain id", async () => {
    const config = new AptosConfig({ network: Network.LOCAL });
    const aptos = new Aptos(config);
    const chainId = await aptos.getChainId();
    expect(chainId).toBe(4);
  });

  test("it fetches block data by block height", async () => {
    const config = new AptosConfig({ network: Network.LOCAL });
    const aptos = new Aptos(config);
    const blockHeight = 1;
    const blockData = await aptos.getBlockByHeight({ blockHeight });
    expect(blockData.block_height).toBe(blockHeight.toString());
  });

  test("it fetches block data by block version", async () => {
    const config = new AptosConfig({ network: Network.LOCAL });
    const aptos = new Aptos(config);
    const blockVersion = 1;
    const blockData = await aptos.getBlockByVersion({ blockVersion });
    expect(blockData.block_height).toBe(blockVersion.toString());
  });

  test("it fetches view function data", async () => {
    const config = new AptosConfig({ network: Network.LOCAL });
    const aptos = new Aptos(config);

    const payload: ViewRequest = {
      function: "0x1::chain_id::get",
      type_arguments: [],
      arguments: [],
    };

    const chainId = await aptos.view({ payload });

    expect(chainId).toBe(4);
  });

  test("it fetches table item data", async () => {
    const config = new AptosConfig({ network: Network.LOCAL });
    const aptos = new Aptos(config);
    const resource = await aptos.getAccountResource({
      accountAddress: "0x1",
      resourceType: "0x1::coin::CoinInfo<0x1::aptos_coin::AptosCoin>",
    });

    const {
      supply: {
        vec: [
          {
            aggregator: {
              vec: [{ handle, key }],
            },
          },
        ],
      },
    } = resource.data as any;

    const supply = await aptos.getTableItem({
      handle,
      data: {
        key_type: "address",
        value_type: "u128",
        key: key,
      },
    });
    expect(parseInt(supply)).toBeGreaterThan(0);
  });

  test("it fetches data with a custom graphql query", async () => {
    const config = new AptosConfig({ network: Network.TESTNET });
    const aptos = new Aptos(config);

    const query: GraphqlQuery = {
      query: `query MyQuery {
        ledger_infos {
          chain_id
        }
      }`,
    };

    const chainId = await aptos.queryIndexer<{
      ledger_infos: [
        {
          chain_id: number;
        },
      ];
    }>({ query });

    expect(chainId.ledger_infos[0].chain_id).toBe(2);
  });
});
