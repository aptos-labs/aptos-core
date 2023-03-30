import { AptosAccount } from "../../account";
import { TokenV2Client } from "../../plugins";
import { AptosClient } from "../../providers";
import { NODE_URL, getFaucetClient, longTestTimeout } from "../unit/test_helper.test";

const client = new AptosClient(NODE_URL);
const faucetClient = getFaucetClient();
const tokenClient = new TokenV2Client(client);

const alice = new AptosAccount();
const bob = new AptosAccount();

const collectionName = "AliceCollection";
const tokenName = "Alice Token";

describe("token objects", () => {
  beforeAll(async () => {
    // Fund both Alice's and Bob's Account
    await faucetClient.fundAccount(alice.address(), 100000000);
    await faucetClient.fundAccount(bob.address(), 100000000);
    console.log("alice", alice.address());
    console.log("bob", bob.address());
  }, longTestTimeout);

  test(
    "create collection",
    async () => {
      await client.waitForTransaction(
        await tokenClient.createCollection(
          alice,
          "Alice's simple collection",
          5,
          collectionName,
          "https://aptos.dev",
          10,
          10,
        ),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "mint",
    async () => {
      await client.waitForTransaction(
        await tokenClient.mint(
          alice,
          collectionName,
          "Alice's simple token",
          tokenName,
          "https://aptos.dev/img/nyan.jpeg",
          ["key"],
          ["bool"],
          ["true"],
        ),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "mint soul bound",
    async () => {
      await client.waitForTransaction(
        await tokenClient.mintSoulBound(
          alice,
          collectionName,
          "Alice's simple soul bound token",
          "Alice's soul bound token",
          "https://aptos.dev/img/nyan.jpeg",
          ["key"],
          ["bool"],
          ["true"],
          bob,
        ),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "burn",
    async () => {
      const tokenAddress = tokenClient.tokenObjectAddress(alice, collectionName, tokenName);
      console.log("token address", tokenAddress);
      await client.waitForTransaction(await tokenClient.burn(alice, tokenAddress.hex()), { checkSuccess: true });
    },
    longTestTimeout,
  );
});
