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
    const res = await this.aptosClient.submitTransaction(signedTxn);
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
      function: "0x1::Token::create_unlimited_token_script",
      type_arguments: [],
      arguments: [
        Buffer.from(collectionName).toString("hex"),
        Buffer.from(name).toString("hex"),
        Buffer.from(description).toString("hex"),
        true,
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
      arguments: [sender, creator, Buffer.from(collectionName).toString("hex"), Buffer.from(name).toString("hex")],
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
      arguments: [receiver, creator, Buffer.from(collectionName).toString("hex"), Buffer.from(name).toString("hex")],
    };
    const transactionHash = await this.submitTransactionHelper(account, payload);
    return transactionHash;
  }

  async getCollectionData(creator: MaybeHexString, collectionName: string): Promise<any> {
    const resources = await this.aptosClient.getAccountResources(creator);
    const accountResource: { type: string; data: any } = resources.find((r) => r.type === "0x1::Token::Collections");
    const { handle }: { handle: string } = accountResource.data.collections;
    const getCollectionTableItemRequest: Types.TableItemRequest = {
      key_type: "0x1::ASCII::String",
      value_type: "0x1::Token::Collection",
      key: collectionName,
    };
    // eslint-disable-next-line no-unused-vars
    const collectionTable = await this.aptosClient.getTableItem(handle, getCollectionTableItemRequest);
    return collectionTable;
  }

  // Retrieve the token's creation_num, which is useful for non-creator operations
  async getTokenData(creator: MaybeHexString, collectionName: string, tokenName: string): Promise<number> {
    const collection: { type: string; data: any } = await this.aptosClient.getAccountResource(
      creator,
      "0x1::Token::Collections",
    );
    const { handle } = collection.data.token_data;
    const tokenId = {
      creator,
      collection: collectionName,
      name: tokenName,
    };

    const getTokenTableItemRequest: Types.TableItemRequest = {
      key_type: "0x1::Token::TokenId",
      value_type: "0x1::Token::TokenData",
      key: tokenId,
    };

    const tableItem = await this.aptosClient.getTableItem(handle, getTokenTableItemRequest);
    return tableItem;
  }

  // Retrieve the token's creation_num, which is useful for non-creator operations
  async getTokenBalance(creator: MaybeHexString, collectionName: string, tokenName: string): Promise<number> {
    const tokenStore: { type: string; data: any } = await this.aptosClient.getAccountResource(
      creator,
      "0x1::Token::TokenStore",
    );
    const { handle } = tokenStore.data.tokens;
    const tokenId = {
      creator,
      collection: collectionName,
      name: tokenName,
    };

    const getTokenTableItemRequest: Types.TableItemRequest = {
      key_type: "0x1::Token::TokenId",
      value_type: "0x1::Token::Token",
      key: tokenId,
    };

    const tableItem = await this.aptosClient.getTableItem(handle, getTokenTableItemRequest);
    return tableItem;
  }
}
