import { AptosAccount } from "../../account";
<<<<<<< HEAD
import { UserTransaction } from "../../generated";
import { AptosToken } from "../../plugins";
import { Provider } from "../../providers";
import { PROVIDER_LOCAL_NETWORK_CONFIG, getFaucetClient, longTestTimeout } from "../unit/test_helper.test";

const provider = new Provider(PROVIDER_LOCAL_NETWORK_CONFIG);
=======
import { AptosToken } from "../../plugins";
import { Provider } from "../../providers";
import { NODE_URL, getFaucetClient, longTestTimeout } from "../unit/test_helper.test";

const provider = new Provider({ fullnodeUrl: NODE_URL, indexerUrl: NODE_URL });
>>>>>>> 7d60beacab (support aptos token)
const faucetClient = getFaucetClient();
const aptosToken = new AptosToken(provider);

const alice = new AptosAccount();
const bob = new AptosAccount();

const collectionName = "AliceCollection";
const tokenName = "Alice Token";
<<<<<<< HEAD
let tokenAddress = "";

describe("token objects", () => {
  beforeAll(async () => {
    // Fund Alice's Account
    await faucetClient.fundAccount(alice.address(), 100000000);
=======

describe("token objects", () => {
  beforeAll(async () => {
    // Fund both Alice's and Bob's Account
    await faucetClient.fundAccount(alice.address(), 100000000);
    await faucetClient.fundAccount(bob.address(), 100000000);
>>>>>>> 7d60beacab (support aptos token)
  }, longTestTimeout);

  test(
    "create collection",
    async () => {
      await provider.waitForTransaction(
<<<<<<< HEAD
        await aptosToken.createCollection(alice, "Alice's simple collection", collectionName, "https://aptos.dev", 5, {
          royaltyNumerator: 10,
          royaltyDenominator: 10,
        }),
=======
        await aptosToken.createCollection(
          alice,
          "Alice's simple collection",
          5,
          collectionName,
          "https://aptos.dev",
          10,
          10,
        ),
>>>>>>> 7d60beacab (support aptos token)
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "mint",
    async () => {
<<<<<<< HEAD
      const txn = await provider.waitForTransactionWithResult(
=======
      await provider.waitForTransaction(
>>>>>>> 7d60beacab (support aptos token)
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
<<<<<<< HEAD
      tokenAddress = (txn as UserTransaction).events[0].data.token;
=======
>>>>>>> 7d60beacab (support aptos token)
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
<<<<<<< HEAD
          bob,
          ["key"],
          ["bool"],
          ["true"],
=======
          ["key"],
          ["bool"],
          ["true"],
          bob,
>>>>>>> 7d60beacab (support aptos token)
        ),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );
<<<<<<< HEAD

  test(
    "freeze transfer",
    async () => {
      await provider.waitForTransaction(await aptosToken.freezeTokenTransafer(alice, tokenAddress), {
        checkSuccess: true,
      });
    },
    longTestTimeout,
  );

  test(
    "unfreeze token transfer",
    async () => {
      await provider.waitForTransaction(await aptosToken.unfreezeTokenTransafer(alice, tokenAddress), {
        checkSuccess: true,
      });
    },
    longTestTimeout,
  );

  test(
    "set token description",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.setTokenDescription(alice, tokenAddress, "my updated token description"),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "set token name",
    async () => {
      await provider.waitForTransaction(await aptosToken.setTokenName(alice, tokenAddress, "my updated token name"), {
        checkSuccess: true,
      });
    },
    longTestTimeout,
  );

  test(
    "set token uri",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.setTokenName(alice, tokenAddress, "https://aptos.dev/img/hero.jpg"),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "add token property",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.addTokenProperty(alice, tokenAddress, "newKey", "BOOLEAN", "true"),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "add typed property",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.addTypedProperty(alice, tokenAddress, "newTypedKey", "VECTOR", "[hello,world]"),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "update typed property",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.updateTypedProperty(alice, tokenAddress, "newTypedKey", "U8", "2"),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "update token property",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.updateTokenProperty(alice, tokenAddress, "newKey", "U8", "5"),
        { checkSuccess: true },
      );
    },
    longTestTimeout,
  );

  test(
    "remove token property",
    async () => {
      await provider.waitForTransaction(await aptosToken.removeTokenProperty(alice, tokenAddress, "newKey"), {
        checkSuccess: true,
      });
    },
    longTestTimeout,
  );

  test(
    "transfer token ownership",
    async () => {
      await provider.waitForTransaction(await aptosToken.transferTokenOwnership(alice, tokenAddress, bob.address()), {
        checkSuccess: true,
      });
    },
    longTestTimeout,
  );

  test(
    "burn token",
    async () => {
      await provider.waitForTransaction(await aptosToken.burnToken(alice, tokenAddress), { checkSuccess: true });
    },
    longTestTimeout,
  );
=======
>>>>>>> 7d60beacab (support aptos token)
});
