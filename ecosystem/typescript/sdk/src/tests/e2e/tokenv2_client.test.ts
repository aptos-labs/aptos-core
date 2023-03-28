import { AptosAccount } from "../../account";
import { bcsSerializeBool } from "../../bcs";
import { TokenV2Client } from "../../plugins";
import { AptosClient } from "../../providers";
import { NODE_URL, getFaucetClient, longTestTimeout } from "../unit/test_helper.test";

test.only(
  "token v2",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = getFaucetClient();
    const tokenClient = new TokenV2Client(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();

    // Fund both Alice's and Bob's Account
    await faucetClient.fundAccount(alice.address(), 100000000);
    await faucetClient.fundAccount(bob.address(), 100000000);
    console.log(alice.address());
    const collectionName = "AliceCollection";

    // Create collection and token on Alice's account
    await client.waitForTransaction(
      await tokenClient.createCollection(
        alice,
        "Alice's simple collection",
        1,
        collectionName,
        "https://aptos.dev",
        10,
        10,
      ),
      { checkSuccess: true },
    );

    const tokenName = "Alice Token";
    await client.waitForTransaction(
      await tokenClient.mint(
        alice,
        collectionName,
        "Alice's simple token",
        tokenName,
        "https://aptos.dev/img/nyan.jpeg",
        ["TOKEN"],
        ["bool"],
        [bcsSerializeBool(true)],
      ),
      { checkSuccess: true },
    );
  },
  longTestTimeout,
);
