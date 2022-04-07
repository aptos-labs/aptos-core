// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import assert from "assert";

import { Account, RestClient, TESTNET_URL, FAUCET_URL, FaucetClient } from "./first_transaction";

export class TokenClient {
    restClient: RestClient;

    constructor(restClient: RestClient) {
        this.restClient = restClient;
    }

    async submitTransactionHelper(account: Account, payload: Record<string, any>) {
        const txn_request = await this.restClient.generateTransaction(account.address(), payload)
        const signed_txn = await this.restClient.signTransaction(account, txn_request)
        const res = await this.restClient.submitTransaction(account, signed_txn)
        await this.restClient.waitForTransaction(res["hash"])
    }

    /** Creates a new collection within the specified account */
    async createCollection(account: Account, description: string, name: string, uri: string) {
        const payload: { function: string; arguments: string[]; type: string; type_arguments: any[] } = {
            type: "script_function_payload",
            function: "0x1::Token::create_unlimited_collection_script",
            type_arguments: [],
            arguments: [
                Buffer.from(description).toString("hex"),
                Buffer.from(name).toString("hex"),
                Buffer.from(uri).toString("hex"),
            ]
        };
        await this.submitTransactionHelper(account, payload);
    }

    async createToken(
        account: Account,
        collection_name: string,
        description: string,
        name: string,
        supply: number,
        uri: string) {
        const payload: { function: string; arguments: string[]; type: string; type_arguments: any[] } = {
            type: "script_function_payload",
            function: "0x1::Token::create_token_script",
            type_arguments: [],
            arguments: [
                Buffer.from(collection_name).toString("hex"),
                Buffer.from(description).toString("hex"),
                Buffer.from(name).toString("hex"),
                supply.toString(),
                Buffer.from(uri).toString("hex")
            ]
        }
        await this.submitTransactionHelper(account, payload);
    }

    async offerToken(
        account: Account,
        receiver: string,
        creator: string,
        token_creation_num: number,
        amount: number) {
        const payload: { function: string; arguments: string[]; type: string; type_arguments: any[] } = {
            type: "script_function_payload",
            function: "0x1::TokenTransfers::offer_script",
            type_arguments: [],
            arguments: [
                receiver,
                creator,
                token_creation_num.toString(),
                amount.toString()
            ]
        }
        await this.submitTransactionHelper(account, payload);
    }

    async claimToken(
        account: Account,
        sender: string,
        creator: string,
        token_creation_num: number) {
        const payload: { function: string; arguments: string[]; type: string; type_arguments: any[] } = {
            type: "script_function_payload",
            function: "0x1::TokenTransfers::claim_script",
            type_arguments: [],
            arguments: [
                sender,
                creator,
                token_creation_num.toString(),
            ]
        }
        await this.submitTransactionHelper(account, payload);
    }

    async cancelTokenOffer(
        account: Account,
        receiver: string,
        creator: string,
        token_creation_num: number) {
        const payload: { function: string; arguments: string[]; type: string; type_arguments: any[] } = {
            type: "script_function_payload",
            function: "0x1::TokenTransfers::cancel_offer_script",
            type_arguments: [],
            arguments: [
                receiver,
                creator,
                token_creation_num.toString()
            ]
        }
        await this.submitTransactionHelper(account, payload);
    }

    /** Retrieve the token's creation_num, which is useful for non-creator operations */
    async getTokenId(creator: string, collection_name: string, token_name: string): Promise<number> {
        const resources = await this.restClient.accountResources(creator);
        let collections = []
        let tokens = []
        for (var resource in resources) {
            if (resources[resource]["type"] == "0x1::Token::Collections") {
                collections = resources[resource]["data"]["collections"]["data"];
            }
        } 
        for (var collection in collections) {
            if (collections[collection]["key"] == collection_name) {
                tokens = collections[collection]["value"]["tokens"]["data"];
            }
        }
        for (var token in tokens) {
            if (tokens[token]["key"] == token_name) {
                return parseInt(tokens[token]["value"]["id"]["creation_num"]);
            }
        }
        assert(false);
    }
  }


async function main() {
    const restClient = new RestClient(TESTNET_URL);
    const client = new TokenClient(restClient);
    const faucet_client = new FaucetClient(FAUCET_URL, restClient);


    const alice = new Account();
    const bob = new Account();

    console.log("\n=== Addresses ===");
    console.log(`Alice: ${alice.address()}. Key Seed: ${Buffer.from(alice.signingKey.secretKey).toString("hex").slice(0, 64)}`);
    console.log(`Bob: ${bob.address()}. Key Seed: ${Buffer.from(bob.signingKey.secretKey).toString("hex").slice(0, 64)}`);

    await faucet_client.fundAccount(alice.address(), 10_000_000);
    await faucet_client.fundAccount(bob.address(), 10_000_000);

    console.log("\n=== Initial Balances ===");
    console.log(`Alice: ${await restClient.accountBalance(alice.address())}`);
    console.log(`Bob: ${await restClient.accountBalance(bob.address())}`);

    await client.createCollection(alice, "Alice's simple collection", "Alice's", "https://aptos.dev");
    await client.createToken(alice, "Alice's", "Alice's simple token", "Alice's first token", 1, "https://aptos.dev/img/nyan.jpeg");

    console.log("\n=== Creating Collection and Token ===");
    const token_id = await client.getTokenId(alice.address(), "Alice's", "Alice's first token");
    console.log(`Alice's token's identifier: ${token_id}`);
    console.log(`See ${restClient.url}/accounts/${alice.address()}/resources`);

    console.log("\n=== Transferring the token to Bob ===")
    await client.offerToken(alice, bob.address(), alice.address(), token_id, 1);
    await client.claimToken(bob, alice.address(), alice.address(), token_id);

    console.log(`See ${restClient.url}/accounts/${bob.address()}/resources`);
}

if (require.main === module) {
    main().then((resp) => console.log(resp));
}