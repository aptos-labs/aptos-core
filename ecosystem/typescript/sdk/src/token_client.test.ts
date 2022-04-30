import { FaucetClient } from "./faucet_client";
import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import { TokenClient } from "./token_client";
import { Types } from "./types";

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

    // Create collection and token on Alice's account
    await tokenClient.createCollection(alice, "Alice's simple collection", "AliceCollection", "https://aptos.dev");
    let resources: Types.AccountResource[] = await client.getAccountResources(alice.address().toString());
    let accountResource: { type: string; data: any } = resources.find((r) => r.type === "0x1::Token::Collections");
    let collection = await tokenClient.tableItem(
        accountResource.data.collections.handle,
        "0x1::ASCII::String",
        "0x1::Token::Collection",
        "AliceCollection",
    );

    expect(collection['name']).toBe("AliceCollection");

    await tokenClient.createToken(
      alice,
      "AliceCollection",
      "Alice's simple token",
      "AliceToken",
      1,
      "https://aptos.dev/img/nyan.jpeg",
    );
    let tokenData = await tokenClient.tableItem(
        collection["tokens"]["handle"],
        "0x1::ASCII::String",
        "0x1::Token::TokenData",
        "AliceToken",
    );
    expect(tokenData['name']).toBe("AliceToken");

    // Transfer Token from Alice's Account to Bob's Account
    const token_id = await tokenClient.getTokenId(alice.address().hex(), "AliceCollection", "AliceToken");
    await tokenClient.offerToken(alice, bob.address().hex(), alice.address().hex(), token_id, 1);
    await tokenClient.claimToken(bob, alice.address().hex(), alice.address().hex(), token_id);

    resources = await client.getAccountResources(bob.address().toString());
    accountResource = resources.find((r) => r.type === "0x1::Token::Gallery");

    let token = await tokenClient.tableItem(
      accountResource.data.gallery.handle,
      "0x1::GUID::ID",
      "0x1::Token::Token",
      {
        "addr": alice.address().toString(),
        "creation_num": `${token_id}`
      },
    );
    expect(token['name']).toBe("AliceToken");
  },
  30 * 1000,
);
