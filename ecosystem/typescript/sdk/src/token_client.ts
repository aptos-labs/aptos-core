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
    const txnRequest = await this.aptosClient.generateTransaction(account.address(), payload, {
      max_gas_amount: "4000",
    });
    const signedTxn = await this.aptosClient.signTransaction(account, txnRequest);
    const res = await this.aptosClient.submitTransaction(account, signedTxn);
    await this.aptosClient.waitForTransaction(res.hash);
    return Promise.resolve(res.hash);
  }

  // Creates a new collection within the specified account
  async createCollection(
    account: AptosAccount,
    name: string,
    description: string,
    uri: string,
  ): Promise<Types.HexEncodedBytes> {
    const payload: Types.TransactionPayload = {
      type: "script_function_payload",
      function: "0x1::Token::create_unlimited_collection_script",
      type_arguments: [],
      arguments: [
        Buffer.from(name).toString("hex"),
        Buffer.from(description).toString("hex"),
        Buffer.from(uri).toString("hex"),
      ],
    };
    const transactionHash = await this.submitTransactionHelper(account, payload);
    return transactionHash;
  }

  // Creates a new token within the specified account
  async createToken(
    account: AptosAccount,
    collectionName: string,
    name: string,
    description: string,
    supply: number,
    uri: string,
  ): Promise<Types.HexEncodedBytes> {
    const payload: Types.TransactionPayload = {
      type: "script_function_payload",
      function: "0x1::Token::create_limited_token_script",
      type_arguments: [],
      arguments: [
        Buffer.from(collectionName).toString("hex"),
        Buffer.from(name).toString("hex"),
        Buffer.from(description).toString("hex"),
        true,
        supply.toString(),
        supply.toString(),
        Buffer.from(uri).toString("hex"),
      ],
    };
    const transactionHash = await this.submitTransactionHelper(account, payload);
    return transactionHash;
  }

  // Offer token to another account
  async offerToken(
    account: AptosAccount,
    receiver: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
    amount: number,
  ): Promise<Types.HexEncodedBytes> {
    const payload: Types.TransactionPayload = {
      type: "script_function_payload",
      function: "0x1::TokenTransfers::offer_script",
      type_arguments: [],
      arguments: [
        receiver,
        creator,
        Buffer.from(collectionName).toString("hex"),
        Buffer.from(name).toString("hex"),
        amount.toString(),
      ],
    };
    const transactionHash = await this.submitTransactionHelper(account, payload);
    return transactionHash;
  }

  // Claim token
  async claimToken(
    account: AptosAccount,
    sender: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
  ): Promise<Types.HexEncodedBytes> {
    const payload: Types.TransactionPayload = {
      type: "script_function_payload",
      function: "0x1::TokenTransfers::claim_script",
      type_arguments: [],
      arguments: [
        sender,
        creator,
        Buffer.from(collectionName).toString("hex"),
        Buffer.from(name).toString("hex"),
      ],
    };
    const transactionHash = await this.submitTransactionHelper(account, payload);
    return transactionHash;
  }

  // Cancel token
  async cancelTokenOffer(
    account: AptosAccount,
    receiver: MaybeHexString,
    creator: MaybeHexString,
    collectionName: string,
    name: string,
  ): Promise<Types.HexEncodedBytes> {
    const payload: Types.TransactionPayload = {
      type: "script_function_payload",
      function: "0x1::TokenTransfers::cancel_offer_script",
      type_arguments: [],
      arguments: [
        sender,
        creator,
        Buffer.from(collectionName).toString("hex"),
        Buffer.from(name).toString("hex"),
      ],
    };
    const transactionHash = await this.submitTransactionHelper(account, payload);
    return transactionHash;
  }

  // Retrieve the token's creation_num, which is useful for non-creator operations
  async getTokenId(creator: MaybeHexString, collectionName: string, tokenName: string): Promise<number> {
    const resources: Types.AccountResource[] = await this.aptosClient.getAccountResources(creator);
    let collections: any[] = [];
    let tokens = [];
    const accountResource: { type: string; data: any } = resources.find((r) => r.type === "0x1::Token::Collections");
    collections = accountResource.data.collections.data;

    for (let index = 0; index < collections.length; index += 1) {
      if (collections[index].key === collectionName) {
        tokens = collections[index].value.tokens.data;
        break;
      }
    }

    for (let index = 0; index < tokens.length; index += 1) {
      if (tokens[index].key === tokenName) {
        return parseInt(tokens[index].value.id.creation_num, 10);
      }
    }

    return null;
  }
}
