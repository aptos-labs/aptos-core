import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import { Types } from "./types";
import { MaybeHexString } from "./hex_string";

export class TokenClient {
    aptosClient: AptosClient;

    constructor(aptosClient: AptosClient) {
        this.aptosClient = aptosClient;
    }

    async submitTransactionHelper(account: AptosAccount, payload: Types.TransactionPayload) {
        const txn_request = await this.aptosClient.generateTransaction(account.address(), payload, {
            max_gas_amount: "4000"
        })
        const signed_txn = await this.aptosClient.signTransaction(account, txn_request)
        const res = await this.aptosClient.submitTransaction(account, signed_txn)
        await this.aptosClient.waitForTransaction(res["hash"])
        return Promise.resolve(res["hash"])
    }

    // Creates a new collection within the specified account 
    async createCollection(account: AptosAccount, description: string, name: string, uri: string): Promise<Types.HexEncodedBytes> {
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
        return await this.submitTransactionHelper(account, payload);
    }

    // Creates a new token within the specified account 
    async createToken(
        account: AptosAccount,
        collection_name: string,
        description: string,
        name: string,
        supply: number,
        uri: string): Promise<Types.HexEncodedBytes> {
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
        return await this.submitTransactionHelper(account, payload);
    }

    // Offer token to another account 
    async offerToken(
        account: AptosAccount,
        receiver: MaybeHexString,
        creator: MaybeHexString,
        token_creation_num: number,
        amount: number): Promise<Types.HexEncodedBytes> {
        const payload: Types.TransactionPayload = {
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
        return await this.submitTransactionHelper(account, payload);
    }

    // Claim token 
    async claimToken(
        account: AptosAccount,
        sender: MaybeHexString,
        creator: MaybeHexString,
        token_creation_num: number): Promise<Types.HexEncodedBytes> {
        const payload: Types.TransactionPayload = {
            type: "script_function_payload",
            function: "0x1::TokenTransfers::claim_script",
            type_arguments: [],
            arguments: [
                sender,
                creator,
                token_creation_num.toString(),
            ]
        }
        return await this.submitTransactionHelper(account, payload);
    }

    // Cancel token 
    async cancelTokenOffer(
        account: AptosAccount,
        receiver: MaybeHexString,
        creator: MaybeHexString,
        token_creation_num: number): Promise<Types.HexEncodedBytes> {
        const payload: Types.TransactionPayload = {
            type: "script_function_payload",
            function: "0x1::TokenTransfers::cancel_offer_script",
            type_arguments: [],
            arguments: [
                receiver,
                creator,
                token_creation_num.toString()
            ]
        }
        return await this.submitTransactionHelper(account, payload);
    }

    // Retrieve the token's creation_num, which is useful for non-creator operations 
    async getTokenId(creator: MaybeHexString, collection_name: string, token_name: string): Promise<number> {
        const resources = await this.aptosClient.getAccountResources(creator);
        let collections = []
        let tokens = []
        let accountResource: {type: string, data: any} = resources.find((r) => r.type === "0x1::Token::Collections");

        collections = accountResource.data.collections.data
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
    }
}
