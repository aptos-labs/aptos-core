// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { AptosAccount, getAddressFromAccountOrAddress } from "../account/aptos_account";
import { AptosClient, OptionalTransactionArgs } from "../providers/aptos_client";
import { MaybeHexString, APTOS_COIN, NetworkToIndexerAPI, NodeAPIToNetwork } from "../utils";
import { TransactionBuilderRemoteABI } from "../transaction_builder";
import { FungibleAssetClient } from "./fungible_asset_client";
import { Provider } from "../providers";
import { AccountAddress } from "../aptos_types";

export const TRANSFER_COINS = "0x1::aptos_account::transfer_coins";
export const COIN_TRANSFER = "0x1::coin::transfer";
/**
 * Class for working with the coin module, such as transferring coins and
 * checking balances.
 */
export class CoinClient {
  aptosClient: AptosClient;

  /**
   * Creates new CoinClient instance
   * @param aptosClient AptosClient instance
   */
  constructor(aptosClient: AptosClient) {
    this.aptosClient = aptosClient;
  }

  /**
   * Generate, sign, and submit a transaction to the Aptos blockchain API to
   * transfer coins from one account to another. By default it transfers
   * 0x1::aptos_coin::AptosCoin, but you can specify a different coin type
   * with the `coinType` argument.
   *
   * You may set `createReceiverIfMissing` to true if you want to create the
   * receiver account if it does not exist on chain yet. If you do not set
   * this to true, the transaction will fail if the receiver account does not
   * exist on-chain.
   *
   * The TS SDK supports fungible assets operations. If you want to use CoinClient
   * with this feature, set the `coinType` to be the fungible asset metadata address.
   * This option uses the `FungibleAssetClient` class and queries the
   * fungible asset primary store.
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
    to: AptosAccount | MaybeHexString,
    amount: number | bigint,
    extraArgs?: OptionalTransactionArgs & {
      // The coin type to use, defaults to 0x1::aptos_coin::AptosCoin.
      // If you want to transfer a fungible asset, set this param to be the
      // fungible asset address
      coinType?: string | MaybeHexString;
      // If set, create the `receiver` account if it doesn't exist on-chain.
      // This is done by calling `0x1::aptos_account::transfer` instead, which
      // will create the account on-chain first if it doesn't exist before
      // transferring the coins to it.
      // If this is the first time an account has received the specified coinType,
      // and this is set to false, the transaction would fail.
      createReceiverIfMissing?: boolean;
    },
  ): Promise<string> {
    // Since we can receive either a fully qualified type tag like "0x1::coin_type::CoinType"
    // or a fungible object address "0x1234...6789" we first check to see if the raw string value includes "::"
    // This is to make sure it's not supposed to be a fungible asset object address.
    const isTypeTag = (extraArgs?.coinType ?? "").toString().includes("::");

    // If the coin type exists, definitely isn't a type tag, and is a valid account address,
    // then we enter this if block under the assumption that it's a fungible asset object address.
    if (extraArgs?.coinType && !isTypeTag && AccountAddress.isValid(extraArgs.coinType)) {
      /* eslint-disable no-console */
      console.warn("to transfer a fungible asset, use `FungibleAssetClient()` class for better support");
      const provider = new Provider({
        fullnodeUrl: this.aptosClient.nodeUrl,
        indexerUrl: NetworkToIndexerAPI[NodeAPIToNetwork[this.aptosClient.nodeUrl]] ?? this.aptosClient.nodeUrl,
      });
      const fungibleAsset = new FungibleAssetClient(provider);
      const txnHash = await fungibleAsset.transfer(
        from,
        extraArgs?.coinType,
        getAddressFromAccountOrAddress(to),
        amount,
      );
      return txnHash;
    }

    // If none is explicitly given, use 0x1::aptos_coin::AptosCoin as the coin type.
    const coinTypeToTransfer = extraArgs?.coinType ?? APTOS_COIN;

    // If we should create the receiver account if it doesn't exist on-chain,
    // use the `0x1::aptos_account::transfer_coins` function.
    let func: string;
    if (extraArgs?.createReceiverIfMissing === undefined) {
      func = TRANSFER_COINS;
    } else {
      func = extraArgs?.createReceiverIfMissing ? TRANSFER_COINS : COIN_TRANSFER;
    }

    // Get the receiver address from the AptosAccount or MaybeHexString.
    const toAddress = getAddressFromAccountOrAddress(to);

    const builder = new TransactionBuilderRemoteABI(this.aptosClient, { sender: from.address(), ...extraArgs });
    const rawTxn = await builder.build(func, [coinTypeToTransfer as string], [toAddress, amount]);

    const bcsTxn = AptosClient.generateBCSTransaction(from, rawTxn);
    const pendingTransaction = await this.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  } // <:!:transfer

  /**
   * Get the balance of the account. By default it checks the balance of
   * 0x1::aptos_coin::AptosCoin, but you can specify a different coin type.
   *
   * to use a different type, set the `coinType` to be the fungible asset type.
   *
   * The TS SDK supports fungible assets operations. If you want to use CoinClient
   * with this feature, set the `coinType` to be the fungible asset metadata address.
   * This option uses the FungibleAssetClient class and queries the
   * fungible asset primary store.
   *
   * @param account Account that you want to get the balance of.
   * @param extraArgs Extra args for checking the balance.
   * @returns Promise that resolves to the balance as a bigint.
   */
  // :!:>checkBalance
  async checkBalance(
    account: AptosAccount | MaybeHexString,
    extraArgs?: {
      // The coin type to use, defaults to 0x1::aptos_coin::AptosCoin.
      // If you want to check the balance of a fungible asset, set this param to be the
      // fungible asset address
      coinType?: string | MaybeHexString;
    },
  ): Promise<bigint> {
    // Since we can receive either a fully qualified type tag like "0x1::coin_type::CoinType"
    // or a fungible object address "0x1234...6789" we first check to see if the raw string value includes "::"
    // This is to make sure it's not supposed to be a fungible asset object address.
    const isTypeTag = (extraArgs?.coinType ?? "").toString().includes("::");

    // If the coin type exists, definitely isn't a type tag, and is a valid account address,
    // then we enter this if block under the assumption that it's a fungible asset object address.
    if (extraArgs?.coinType && !isTypeTag && AccountAddress.isValid(extraArgs.coinType)) {
      /* eslint-disable no-console */
      console.warn("to check balance of a fungible asset, use `FungibleAssetClient()` class for better support");
      const provider = new Provider({
        fullnodeUrl: this.aptosClient.nodeUrl,
        indexerUrl: NetworkToIndexerAPI[NodeAPIToNetwork[this.aptosClient.nodeUrl]] ?? this.aptosClient.nodeUrl,
      });
      const fungibleAsset = new FungibleAssetClient(provider);
      const balance = await fungibleAsset.getPrimaryBalance(
        getAddressFromAccountOrAddress(account),
        extraArgs?.coinType,
      );
      return balance;
    }

    const coinType = extraArgs?.coinType ?? APTOS_COIN;
    const typeTag = `0x1::coin::CoinStore<${coinType}>`;
    const address = getAddressFromAccountOrAddress(account);
    const accountResource = await this.aptosClient.getAccountResource(address, typeTag);
    return BigInt((accountResource.data as any).coin.value);
  } // <:!:checkBalance
}
