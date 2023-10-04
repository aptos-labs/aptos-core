import { AptosAccount } from "../../account";
import { UserTransaction, WriteResource, WriteSetChange_WriteResource } from "../../generated";
import { AptosToken } from "../../plugins";
import { Provider } from "../../providers";
import { PROVIDER_LOCAL_NETWORK_CONFIG, getFaucetClient, longTestTimeout } from "../unit/test_helper.test";

const provider = new Provider(PROVIDER_LOCAL_NETWORK_CONFIG);
const faucetClient = getFaucetClient();
const aptosToken = new AptosToken(provider);

const alice = new AptosAccount();
const bob = new AptosAccount();

const collectionName = "AliceCollection";
const tokenName = "Alice Token";
let tokenAddress = "";
let collectionAddress = "";

describe("token objects", () => {
  beforeAll(async () => {
    // Fund Alice's Account
    await faucetClient.fundAccount(alice.address(), 100000000);
    await faucetClient.fundAccount(bob.address(), 100000000);
  }, longTestTimeout);

  test(
    "create collection",
    async () => {
      const txn = await provider.waitForTransactionWithResult(
        await aptosToken.createCollection(alice, "Alice's simple collection", collectionName, "https://aptos.dev", 5),
        { checkSuccess: true },
      );
      const objectCore = (txn as UserTransaction).changes.find(
        (change) => (change as WriteResource).data.type === "0x1::object::ObjectCore",
      );
      collectionAddress = (objectCore as WriteSetChange_WriteResource).address;
    },
    longTestTimeout,
  );

  test(
    "mint",
    async () => {
      const txn = await provider.waitForTransactionWithResult(
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
      tokenAddress = (txn as UserTransaction).events[0].data.token;
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
    "transfer non fungible token",
    async () => {
      const getTokenDataSpy = jest.spyOn(provider, "getTokenData");
      const getTokenDataSpyResponse = { current_token_datas_v2: new Array() };
      getTokenDataSpyResponse.current_token_datas_v2.push({ is_fungible_v2: undefined });
      getTokenDataSpy.mockResolvedValue(getTokenDataSpyResponse);

      await provider.waitForTransaction(
        await aptosToken.transfer({ owner: bob, tokenAddress, recipient: alice.address() }),
        {
          checkSuccess: true,
        },
      );
      getTokenDataSpy.mockRestore();
    },
    longTestTimeout,
  );

  test(
    "transfer non fungible token when isFungibleToken param set to false",
    async () => {
      await provider.waitForTransaction(
        await aptosToken.transfer({ owner: alice, tokenAddress, recipient: bob.address() }, false),
        {
          checkSuccess: true,
        },
      );
    },
    longTestTimeout,
  );

  test(
    "getTokenData indexer query is not being called when isFungibleToken param is set",
    async () => {
      const getTokenDataSpy = jest.spyOn(provider, "getTokenData");
      await aptosToken.transfer({ owner: bob, tokenAddress, recipient: alice.address() }, false);
      expect(getTokenDataSpy).not.toBeCalled();
      getTokenDataSpy.mockRestore();
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

  test(
    "burn object",
    async () => {
      await provider.waitForTransaction(await aptosToken.burnObject(alice, collectionAddress), { checkSuccess: true });
    },
    longTestTimeout,
  );
});
