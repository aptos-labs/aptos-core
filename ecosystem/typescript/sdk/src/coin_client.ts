// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from "./aptos_account";
import { AptosClient } from "./aptos_client";
import * as Gen from "./generated/index";
import { HexString } from "./hex_string";
import { BCS, TransactionBuilderABI } from "./transaction_builder";
import { COIN_ABIS } from "./abis";

export const APTOS_COIN = "0x1::aptos_coin::AptosCoin";

/**
 * Class for working with the coin module, such as transferring coins and
 * checking balances.
 */
export class CoinClient {
  aptosClient: AptosClient;

  transactionBuilder: TransactionBuilderABI;

  /**
   * Creates new CoinClient instance
   * @param aptosClient AptosClient instance
   */
  constructor(aptosClient: AptosClient) {
    this.aptosClient = aptosClient;
    this.transactionBuilder = new TransactionBuilderABI(COIN_ABIS.map((abi) => new HexString(abi).toUint8Array()));
  }

  /**
   * Generate, submit, and wait for a transaction to transfer AptosCoin from
   * one account to another.
   *
   * If the transaction is submitted successfully, it returns the response
   * from the API indicating that the transaction was submitted.
   *
   * @param from Account sending the coins
   * @param from Account to receive the coins
   * @param amount Number of coins to transfer
   * @param extraArgs Extra args for building the transaction or configuring how
   * the client should submit and wait for the transaction.
   * @returns Promise that resolves to the response from the API
   */
  async transfer(
    from: AptosAccount,
    to: AptosAccount,
    amount: number | bigint,
    extraArgs?: {
      // The coin type to use, defaults to 0x1::aptos_coin::AptosCoin
      coinType?: string;
      maxGasAmount?: BCS.Uint64;
      gasUnitPrice?: BCS.Uint64;
      expireTimestamp?: BCS.Uint64;
      // If true, this function will throw if the transaction is not committed succesfully.
      checkSuccess?: boolean;
    },
  ): Promise<Gen.Transaction> {
    const coinTypeToTransfer = extraArgs?.coinType ?? APTOS_COIN;
    const payload = this.transactionBuilder.buildTransactionPayload(
      "0x1::coin::transfer",
      [coinTypeToTransfer],
      [to.address(), amount],
    );
    return this.aptosClient.generateSignSendWaitForTransaction(from, payload, extraArgs);
  }

  /**
   * Generate, submit, and wait for a transaction to transfer AptosCoin from
   * one account to another.
   *
   * If the transaction is submitted successfully, it returns the response
   * from the API indicating that the transaction was submitted.
   *
   * @param account Account that you want to check the balance of.
   * @param extraArgs Extra args for checking the balance.
   * @returns Promise that resolves to the balance as a bigint.
   */
  async checkBalance(
    account: AptosAccount,
    extraArgs?: {
      // The coin type to use, defaults to 0x1::aptos_coin::AptosCoin
      coinType?: string;
    },
  ): Promise<bigint> {
    const coinType = extraArgs?.coinType ?? APTOS_COIN;
    const typeTag = `0x1::coin::CoinStore<${coinType}>`;
    const resources = await this.aptosClient.getAccountResources(account.address());
    const accountResource = resources.find((r) => r.type === typeTag);
    return BigInt((accountResource!.data as any).coin.value);
  }
}
