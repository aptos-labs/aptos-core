import { AptosAccount } from "../../account/aptos_account";
import { bcsSerializeBool } from "../../bcs";
import { TokenClient } from "../../plugins/token_client";
import { getFaucetClient, longTestTimeout } from "../unit/test_helper.test";
import { Network, sleep } from "../../utils";
import { Provider } from "../../providers";
import { AptosToken } from "../../plugins";

const provider = new Provider(Network.LOCAL);
const faucetClient = getFaucetClient();
const tokenClient = new TokenClient(provider.aptosClient);
const aptosToken = new AptosToken(provider);
const alice = new AptosAccount();
const collectionName = "AliceCollection";
const collectionNameV2 = "AliceCollection2";
const tokenName = "Alice Token";

let skipTest = false;
let runTests = describe;

describe("Indexer", () => {
  it("should throw an error when account address is not valid", async () => {
    const address1 = "702ca08576f66393140967fef983bb6bf160dafeb73de9c4ddac4d2dc";
    expect(async () => {
      await provider.getOwnedTokens(address1);
    }).rejects.toThrow(`${address1} is less than 66 chars long.`);

    const address2 = "0x702ca08576f66393140967fef983bb6bf160dafeb73de9c4ddac4d2dc";
    expect(async () => {
      await provider.getOwnedTokens(address2);
    }).rejects.toThrow(`${address2} is less than 66 chars long.`);
  });

  it("should not throw an error when account address is missing 0x", async () => {
    const address = "790a34c702ca08576f66393140967fef983bb6bf160dafeb73de9c4ddac4d2dc";
    expect(async () => {
      await provider.getOwnedTokens(address);
    }).not.toThrow();
  });

  beforeAll(async () => {
    const indexerLedgerInfo = await provider.getIndexerLedgerInfo();
    const fullNodeChainId = await provider.getChainId();

    console.log(
      `\n network chain id is: ${fullNodeChainId}, indexer chain id is: ${indexerLedgerInfo.ledger_infos[0].chain_id}`,
    );

    if (indexerLedgerInfo.ledger_infos[0].chain_id !== fullNodeChainId) {
      console.log(`\n network chain id and indexer chain id are not synced, skipping rest of tests`);
      skipTest = true;
      runTests = describe.skip;
    } else {
      console.log(`\n network chain id and indexer chain id are in synced, running tests`);
    }

    if (!skipTest) {
      await faucetClient.fundAccount(alice.address(), 100000000);
      // Create collection and token V1 on Alice's account
      await provider.waitForTransaction(
        await tokenClient.createCollection(alice, collectionName, "Alice's simple collection", "https://aptos.dev"),
        { checkSuccess: true },
      );
      await provider.waitForTransaction(
        await tokenClient.createTokenWithMutabilityConfig(
          alice,
          collectionName,
          tokenName,
          "Alice's simple token",
          1,
          "https://aptos.dev/img/nyan.jpeg",
          1000,
          alice.address(),
          1,
          0,
          ["TOKEN_BURNABLE_BY_OWNER"],
          [bcsSerializeBool(true)],
          ["bool"],
          [false, false, false, false, true],
        ),
        { checkSuccess: true },
      );

      // Create collection and token V2 on Alice's account
      await provider.waitForTransaction(
        await aptosToken.createCollection(
          alice,
          "Alice's simple collection",
          collectionNameV2,
          "https://aptos.dev",
          5,
          {
            royaltyNumerator: 10,
            royaltyDenominator: 10,
          },
        ),
        { checkSuccess: true },
      );

      await provider.waitForTransactionWithResult(
        await aptosToken.mint(
          alice,
          collectionNameV2,
          "Alice's simple token",
          tokenName,
          "https://aptos.dev/img/nyan.jpeg",
          ["key"],
          ["bool"],
          ["true"],
        ),
        { checkSuccess: true },
      );
    }
  }, longTestTimeout);

  runTests("get data", () => {
    jest.retryTimes(5);
    beforeEach(async () => {
      await sleep(1000);
    });

    it("gets indexer ledger info", async () => {
      const ledgerInfo = await provider.getIndexerLedgerInfo();
      expect(ledgerInfo.ledger_infos[0].chain_id).toBeGreaterThan(1);
    });

    // OBJECTS //

    it(
      "gets account owned objects data",
      async () => {
        const accountObjects = await provider.getAccountOwnedObjects(alice.address().hex());
        expect(accountObjects.current_objects.length).toBe(2);
      },
      longTestTimeout,
    );

    // TOKENS //

    it(
      "gets token activities",
      async () => {
        const accountTokens = await provider.getOwnedTokens(alice.address().hex());
        expect(accountTokens.current_token_ownerships_v2).toHaveLength(2);
        // V1
        const tokenActivityV1 = await provider.getTokenActivities(
          accountTokens.current_token_ownerships_v2[0].current_token_data!.token_data_id,
        );
        expect(tokenActivityV1.token_activities_v2).toHaveLength(2);

        expect(tokenActivityV1.token_activities_v2[0].token_standard).toEqual("v1");
        expect(tokenActivityV1.token_activities_v2[0].from_address).toEqual(alice.address().hex());

        expect(tokenActivityV1.token_activities_v2[1].token_standard).toEqual("v1");
        expect(tokenActivityV1.token_activities_v2[1].to_address).toEqual(alice.address().hex());
        // V2
        const tokenActivityV2 = await provider.getTokenActivities(
          accountTokens.current_token_ownerships_v2[1].current_token_data!.token_data_id,
        );
        expect(tokenActivityV2.token_activities_v2).toHaveLength(1);
        expect(tokenActivityV2.token_activities_v2[0].token_standard).toEqual("v2");
        expect(tokenActivityV2.token_activities_v2[0].from_address).toEqual(alice.address().hex());
      },
      longTestTimeout,
    );

    it(
      "gets token activities count",
      async () => {
        const accountTokens = await provider.getOwnedTokens(alice.address().hex(), {
          orderBy: [{ last_transaction_version: "desc" }],
        });
        expect(accountTokens.current_token_ownerships_v2[0].token_standard).toBe("v2");
        expect(accountTokens.current_token_ownerships_v2[1].token_standard).toBe("v1");
        const tokenActivitiesV2Count = await provider.getTokenActivitiesCount(
          accountTokens.current_token_ownerships_v2[0].current_token_data!.token_data_id,
        );
        expect(tokenActivitiesV2Count.token_activities_v2_aggregate.aggregate?.count).toBe(1);
        const tokenActivitiesV1Count = await provider.getTokenActivitiesCount(
          accountTokens.current_token_ownerships_v2[1].current_token_data!.token_data_id,
        );
        expect(tokenActivitiesV1Count.token_activities_v2_aggregate.aggregate?.count).toBe(2);
      },
      longTestTimeout,
    );

    it(
      "gets account token count",
      async () => {
        const accountTokenCount = await provider.getAccountTokensCount(alice.address().hex());
        expect(accountTokenCount.current_token_ownerships_v2_aggregate.aggregate?.count).toEqual(2);
      },
      longTestTimeout,
    );

    it(
      "gets token data",
      async () => {
        const accountTokens = await provider.getOwnedTokens(alice.address().hex());
        const tokenData = await provider.getTokenData(
          accountTokens.current_token_ownerships_v2[0].current_token_data!.token_data_id,
        );
        expect(tokenData.current_token_datas_v2[0].token_name).toEqual(tokenName);
      },
      longTestTimeout,
    );

    it(
      "gets token owners data",
      async () => {
        const accountTokens = await provider.getOwnedTokens(alice.address().hex());
        const tokenOwnersData = await provider.getTokenOwnersData(
          accountTokens.current_token_ownerships_v2[0].current_token_data!.token_data_id,
          0,
        );
        expect(tokenOwnersData.current_token_ownerships_v2[0].owner_address).toEqual(alice.address().hex());
      },
      longTestTimeout,
    );

    it(
      "gets token current owner data",
      async () => {
        const accountTokens = await provider.getOwnedTokens(alice.address().hex());
        const tokenOwnersData = await provider.getTokenCurrentOwnerData(
          accountTokens.current_token_ownerships_v2[0].current_token_data!.token_data_id,
          0,
        );
        expect(tokenOwnersData.current_token_ownerships_v2[0].owner_address).toEqual(alice.address().hex());
      },
      longTestTimeout,
    );

    it("gets account owned tokens", async () => {
      const tokens = await provider.getOwnedTokens(alice.address().hex());
      expect(tokens.current_token_ownerships_v2.length).toEqual(2);
    });

    it("gets account owned tokens by token data id", async () => {
      const accountTokens = await provider.getOwnedTokens(alice.address().hex());

      const tokens = await provider.getOwnedTokensByTokenData(
        accountTokens.current_token_ownerships_v2[0].current_token_data!.token_data_id,
      );
      expect(tokens.current_token_ownerships_v2).toHaveLength(1);
      expect(tokens.current_token_ownerships_v2[0].owner_address).toEqual(alice.address().hex());
    });

    it("gets account tokens owned from collection address", async () => {
      const collectionAddress = await provider.getCollectionAddress(alice.address().hex(), collectionNameV2);
      const tokensFromCollectionAddress = await provider.getTokenOwnedFromCollectionAddress(
        alice.address().hex(),
        collectionAddress,
      );
      expect(tokensFromCollectionAddress.current_token_ownerships_v2[0].current_token_data!.token_name).toEqual(
        tokenName,
      );
    });

    it(
      "gets account current tokens by collection name and creator address",
      async () => {
        const tokens = await provider.getTokenOwnedFromCollectionNameAndCreatorAddress(
          alice.address().hex(),
          collectionNameV2,
          alice.address().hex(),
        );
        expect(tokens.current_token_ownerships_v2).toHaveLength(1);
        expect(tokens.current_token_ownerships_v2[0].current_token_data?.token_name).toEqual(tokenName);
      },
      longTestTimeout,
    );

    it(
      "returns same result for getTokenOwnedFromCollectionNameAndCreatorAddress and getTokenOwnedFromCollectionAddress",
      async () => {
        const collectionAddress = await provider.getCollectionAddress(alice.address().hex(), collectionNameV2);
        const tokensFromCollectionAddress = await provider.getTokenOwnedFromCollectionAddress(
          alice.address().hex(),
          collectionAddress,
        );
        const tokensFromNameAndCreatorAddress = await provider.getTokenOwnedFromCollectionNameAndCreatorAddress(
          alice.address().hex(),
          collectionNameV2,
          alice.address().hex(),
        );

        expect(tokensFromCollectionAddress.current_token_ownerships_v2).toEqual(
          tokensFromNameAndCreatorAddress.current_token_ownerships_v2,
        );
      },
      longTestTimeout,
    );

    it("gets the collection data", async () => {
      const collectionData = await provider.getCollectionData(alice.address().hex(), collectionName);
      expect(collectionData.current_collections_v2).toHaveLength(1);
      expect(collectionData.current_collections_v2[0].collection_name).toEqual(collectionName);
    });

    it("gets the currect collection address", async () => {
      const collectionData = await provider.getCollectionData(alice.address().hex(), collectionNameV2);
      const collectionAddress = await provider.getCollectionAddress(alice.address().hex(), collectionNameV2);
      expect(collectionData.current_collections_v2[0].collection_id).toEqual(collectionAddress);
    });

    it(
      "queries for all collections that an account has tokens for",
      async () => {
        const collections = await provider.getCollectionsWithOwnedTokens(alice.address().hex());
        expect(collections.current_collection_ownership_v2_view.length).toEqual(2);
      },
      longTestTimeout,
    );

    // TRANSACTIONS //
    it(
      "gets account transactions count",
      async () => {
        const accountTransactionsCount = await provider.getAccountTransactionsCount(alice.address().hex());
        expect(accountTransactionsCount.account_transactions_aggregate.aggregate?.count).toEqual(5);
      },
      longTestTimeout,
    );

    it(
      "gets account transactions data",
      async () => {
        const accountTransactionsData = await provider.getAccountTransactionsData(alice.address().hex());
        expect(accountTransactionsData.account_transactions.length).toEqual(5);
        expect(accountTransactionsData.account_transactions[0]).toHaveProperty("transaction_version");
      },
      longTestTimeout,
    );

    it(
      "gets top user transactions",
      async () => {
        const topUserTransactions = await provider.getTopUserTransactions(5);
        expect(topUserTransactions.user_transactions.length).toEqual(5);
      },
      longTestTimeout,
    );

    it(
      "gets user transactions",
      async () => {
        const userTransactions = await provider.getUserTransactions({ options: { limit: 4 } });
        expect(userTransactions.user_transactions.length).toEqual(4);
      },
      longTestTimeout,
    );

    it(
      "gets number of delegators",
      async () => {
        const numberOfDelegators = await provider.getNumberOfDelegators(alice.address().hex());
        expect(numberOfDelegators.num_active_delegator_per_pool).toHaveLength(0);
      },
      longTestTimeout,
    );
    // ACCOUNT //

    it(
      "gets account coin data",
      async () => {
        const accountCoinData = await provider.getAccountCoinsData(alice.address().hex());
        expect(accountCoinData.current_fungible_asset_balances[0].asset_type).toEqual("0x1::aptos_coin::AptosCoin");
      },
      longTestTimeout,
    );

    it(
      "gets account coin data count",
      async () => {
        const accountCoinDataCount = await provider.getAccountCoinsDataCount(alice.address().hex());
        expect(accountCoinDataCount.current_fungible_asset_balances_aggregate.aggregate?.count).toEqual(1);
      },
      longTestTimeout,
    );

    // TOKEN STANDARD FILTER //

    it("gets account owned tokens with a specified token standard", async () => {
      const tokens = await provider.getOwnedTokens(alice.address().hex(), { tokenStandard: "v2" });
      expect(tokens.current_token_ownerships_v2).toHaveLength(1);
    });

    it(
      "gets account current tokens of a specific collection by the collection address with token standard specified",
      async () => {
        const tokens = await provider.getTokenOwnedFromCollectionNameAndCreatorAddress(
          alice.address().hex(),
          collectionNameV2,
          alice.address().hex(),
          {
            tokenStandard: "v2",
          },
        );
        expect(tokens.current_token_ownerships_v2).toHaveLength(1);
        expect(tokens.current_token_ownerships_v2[0].token_standard).toEqual("v2");
      },
      longTestTimeout,
    );

    it(
      "queries for all v2 collections that an account has tokens for",
      async () => {
        const collections = await provider.getCollectionsWithOwnedTokens(alice.address().hex(), {
          tokenStandard: "v2",
        });
        expect(collections.current_collection_ownership_v2_view.length).toEqual(1);
      },
      longTestTimeout,
    );

    // ORDER BY //

    it("gets account owned tokens sorted desc by token standard", async () => {
      const tokens = await provider.getOwnedTokens(alice.address().hex(), {
        orderBy: [{ token_standard: "desc" }],
      });
      expect(tokens.current_token_ownerships_v2).toHaveLength(2);
      expect(tokens.current_token_ownerships_v2[0].token_standard).toEqual("v2");
    });

    it("gets account owned tokens sorted desc by collection name", async () => {
      const tokens = await provider.getOwnedTokens(alice.address().hex(), {
        orderBy: [{ current_token_data: { current_collection: { collection_name: "desc" } } }],
      });
      expect(tokens.current_token_ownerships_v2).toHaveLength(2);
      expect(tokens.current_token_ownerships_v2[0].token_standard).toEqual("v2");
    });

    it("gets token activities sorted desc by collection name", async () => {
      const accountTokens = await provider.getOwnedTokens(alice.address().hex());
      expect(accountTokens.current_token_ownerships_v2).toHaveLength(2);
      expect(accountTokens.current_token_ownerships_v2[0].token_standard).toEqual("v1");

      const tokens = await provider.getTokenActivities(
        accountTokens.current_token_ownerships_v2[0].current_token_data!.token_data_id,
        {
          orderBy: [{ transaction_version: "desc" }],
        },
      );
      expect(tokens.token_activities_v2).toHaveLength(2);
      expect(tokens.token_activities_v2[0].token_standard).toEqual("v1");
    });

    it(
      "gets account transactions data",
      async () => {
        const accountTransactionsData = await provider.getAccountTransactionsData(alice.address().hex(), {
          orderBy: [{ transaction_version: "desc" }],
        });
        expect(accountTransactionsData.account_transactions.length).toEqual(5);
        expect(accountTransactionsData.account_transactions[0]).toHaveProperty("transaction_version");
      },
      longTestTimeout,
    );
  });
});
