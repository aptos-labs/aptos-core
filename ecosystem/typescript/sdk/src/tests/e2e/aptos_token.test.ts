import { AptosAccount } from "../../account";
import { AptosToken } from "../../plugins";
import { Provider } from "../../providers";
import { NODE_URL, getFaucetClient, longTestTimeout } from "../unit/test_helper.test";

const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
const faucetClient = getFaucetClient();
const aptosToken = new AptosToken(provider);

const alice = new AptosAccount();
const bob = new AptosAccount();

const collectionName = "AliceCollection";
const tokenName = "Alice Token";

describe("token objects", () => {
  beforeAll(async () => {
    // Fund both Alice's and Bob's Account
    await faucetClient.fundAccount(alice.address(), 100000000);
    await faucetClient.fundAccount(bob.address(), 100000000);
  }, longTestTimeout);

  test(
    "create collection",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.createCollection(
          alice,
          "Alice's simple collection",
          collectionName,
          "https://aptos.dev",
          5,
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
      await provider.waitForTransaction(
        await aptosToken.mint(
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
      await provider.waitForTransaction(
        await aptosToken.mintSoulBound(
          alice,
          collectionName,
          "Alice's simple soul bound token",
          "Alice's soul bound token",
          "https://aptos.dev/img/nyan.jpeg",
          bob,
          ["key"],
          ["bool"],
          ["true"],
        ),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );
});
