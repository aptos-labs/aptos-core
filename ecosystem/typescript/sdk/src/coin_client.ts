// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { AptosAccount } from "./aptos_account";
import { AptosClient, OptionalTransactionArgs } from "./aptos_client";
import { HexString } from "./hex_string";
import { TransactionBuilderABI } from "./transaction_builder";
import { COIN_ABIS } from "./abis";
import { APTOS_COIN } from "./utils";

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
   * Generate, sign, and submit a transaction to the Aptos blockchain API to
   * transfer AptosCoin from one account to another.
   *
   * @param from Account sending the coins
   * @param to Account to receive the coins
   * @param amount Number of coins to transfer
   * @param extraArgs Extra args for building the transaction or configuring how
   * the client should submit and wait for the transaction
   * @returns The hash of the transaction submitted to the API
   */
  // :!:>transfer
  async transfer(
    from: AptosAccount,
    to: AptosAccount,
    amount: number | bigint,
    extraArgs?: OptionalTransactionArgs & {
      // The coin type to use, defaults to 0x1::aptos_coin::AptosCoin
      coinType?: string;
    },
  ): Promise<string> {
    const coinTypeToTransfer = extraArgs?.coinType ?? APTOS_COIN;
    const payload = this.transactionBuilder.buildTransactionPayload(
      "0x1::coin::transfer",
      [coinTypeToTransfer],
      [to.address(), amount],
    );
    return this.aptosClient.generateSignSubmitTransaction(from, payload, extraArgs);
  } // <:!:transfer

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
  // :!:>checkBalance
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
  } // <:!:checkBalance
}
