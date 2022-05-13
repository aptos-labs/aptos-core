import { FaucetClient } from "./faucet_client";
import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import { TokenClient } from "./token_client";

import { NODE_URL, FAUCET_URL } from "./util.test";

test(
  "full tutorial nft token flow",
  async () => {
    const client = new AptosClient(NODE_URL);
    const faucetClient = new FaucetClient(NODE_URL, FAUCET_URL);
    const tokenClient = new TokenClient(client);

    const alice = new AptosAccount();
    const bob = new AptosAccount();

    // Fund both Alice's and Bob's Account
    await faucetClient.fundAccount(alice.address(), 10000);
    await faucetClient.fundAccount(bob.address(), 5000);

    const collection_name = "AliceCollection";
    const token_name = "Alice Token";

    // Create collection and token on Alice's account
    await tokenClient.createCollection(alice, collection_name, "Alice's simple collection", "https://aptos.dev");

    await tokenClient.createToken(
      alice,
      collection_name,
      token_name,
      "Alice's simple token",
      1,
      "https://aptos.dev/img/nyan.jpeg",
    );

    // Transfer Token from Alice's Account to Bob's Account
    await tokenClient.getCollectionData(alice.address().hex(), collection_name);
    await tokenClient.getTokenBalance(alice.address().hex(), collection_name, token_name);
    await tokenClient.getTokenData(alice.address().hex(), collection_name, token_name);
    await tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collection_name, token_name, 1);
    await tokenClient.cancelTokenOffer(alice, bob.address().hex(), alice.address().hex(), collection_name, token_name);
    await tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), collection_name, token_name, 1);
    await tokenClient.claimToken(bob, alice.address().hex(), alice.address().hex(), collection_name, token_name);
  },
  30 * 1000,
);
