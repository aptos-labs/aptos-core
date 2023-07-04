// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/* eslint-disable max-len */

import {
  AptosClient,
  AptosAccount,
  HexString,
  MaybeHexString,
  OptionalTransactionArgs,
  Provider,
  TransactionBuilderRemoteABI,
} from "aptos";

type TransactionPayload = {
  type: string;
  function: string;
  type_arguments: string[];
  arguments: any[];
};

const APTOS_COIN: string = "0x1::aptos_coin::AptosCoin";
const COIN_LISTING: string = "coin_listing";
const COLLECTION_OFFER: string = "collection_offer";
const FEE_SCHEDULE: string = "fee_schedule";
const LISTING: string = "listing";

/**
 * Class for managing the example marketplace.  It builds payloads to be used with the wallet adapter, but can
 * also submit payloads directly with an AptosAccount.
 */
export class Marketplace {
  readonly provider: Provider;
  readonly code_location: HexString;

  constructor(provider: Provider, code_location: MaybeHexString) {
    this.provider = provider;
    this.code_location = HexString.ensure(code_location);
  }

  // Coin listing operations
  async buildInitFixedPriceListing(
    object: MaybeHexString,
    feeSchedule: MaybeHexString,
    startTime: bigint,
    price: bigint,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COIN_LISTING,
      "init_fixed_price",
      [coin],
      [HexString.ensure(object).hex(), HexString.ensure(feeSchedule).hex(), startTime.toString(10), price.toString(10)],
    );
  }

  async initAuctionListing(
    object: MaybeHexString,
    feeSchedule: MaybeHexString,
    startTime: bigint,
    startingBid: bigint,
    bidIncrement: bigint,
    auctionEndTime: bigint,
    minimumBidTimeBeforeEnd: bigint,
    buyItNowPrice?: bigint,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COIN_LISTING,
      "init_auction",
      [coin],
      [
        HexString.ensure(object).hex(),
        HexString.ensure(feeSchedule).hex(),
        startTime.toString(10),
        startingBid.toString(10),
        bidIncrement.toString(10),
        auctionEndTime.toString(10),
        minimumBidTimeBeforeEnd.toString(10),
        buyItNowPrice?.toString(10),
      ],
    );
  }

  async initFixedPriceListingForTokenv1(
    tokenCreator: MaybeHexString,
    tokenCollection: string,
    tokenName: string,
    tokenPropertyVersion: bigint,
    feeSchedule: MaybeHexString,
    startTime: bigint,
    price: bigint,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COIN_LISTING,
      "init_fixed_price_for_tokenv1",
      [coin],
      [
        HexString.ensure(tokenCreator).hex(),
        tokenCollection,
        tokenName,
        tokenPropertyVersion.toString(10),
        HexString.ensure(feeSchedule).hex(),
        startTime.toString(10),
        price.toString(10),
      ],
    );
  }

  async initAuctionListingForTokenv1(
    tokenCreator: MaybeHexString,
    tokenCollection: string,
    tokenName: string,
    feeSchedule: MaybeHexString,
    startTime: bigint,
    startingBid: bigint,
    bidIncrement: bigint,
    auctionEndTime: bigint,
    minimumBidTimeBeforeEnd: bigint,
    buyItNowPrice?: bigint,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COIN_LISTING,
      "init_auction_for_tokenv1",
      [coin],
      [
        HexString.ensure(tokenCreator).hex(),
        tokenCollection,
        tokenName,
        HexString.ensure(feeSchedule).hex(),
        startTime.toString(10),
        startingBid.toString(10),
        bidIncrement.toString(10),
        auctionEndTime.toString(10),
        minimumBidTimeBeforeEnd.toString(10),
        buyItNowPrice?.toString(10),
      ],
    );
  }

  async purchaseListing(listing: MaybeHexString, coin: string = APTOS_COIN): Promise<TransactionPayload> {
    return this.buildTransactionPayload(COIN_LISTING, "purchase", [coin], [HexString.ensure(listing).hex()]);
  }

  async endFixedPriceListing(listing: MaybeHexString, coin: string = APTOS_COIN): Promise<TransactionPayload> {
    return this.buildTransactionPayload(COIN_LISTING, "end_fixed_price", [coin], [HexString.ensure(listing).hex()]);
  }

  async bidAuctionListing(
    bidder: MaybeHexString,
    listing: MaybeHexString,
    bid_amount: bigint,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COIN_LISTING,
      "bid",
      [coin],
      [HexString.ensure(listing).hex(), bid_amount.toString(10)],
    );
  }

  async completeAuctionListing(
    listing: MaybeHexString,
    bid_amount: bigint,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(COIN_LISTING, "complete_auction", [coin], [HexString.ensure(listing).hex()]);
  }

  // Listing operations
  async extract_tokenv1(object: MaybeHexString): Promise<TransactionPayload> {
    return this.buildTransactionPayload(LISTING, "extract_tokenv1", [], [HexString.ensure(object).hex()]);
  }

  // Collection offer operations

  async initCollectionOfferForTokenv1(
    tokenCreator: MaybeHexString,
    tokenCollection: string,
    feeSchedule: MaybeHexString,
    price: bigint,
    amount: bigint,
    expiration_time: bigint, // TODO: convert to time?
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COLLECTION_OFFER,
      "init_for_tokenv1_entry",
      [coin],
      [
        HexString.ensure(tokenCreator).hex(),
        tokenCollection,
        HexString.ensure(feeSchedule).hex(),
        price,
        amount,
        expiration_time,
      ],
    );
  }

  async initCollectionOfferForTokenv2(
    collection: MaybeHexString,
    feeSchedule: MaybeHexString,
    price: bigint,
    amount: bigint,
    expiration_time: bigint, // TODO: convert to time?
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COLLECTION_OFFER,
      "init_for_tokenv2_entry",
      [coin],
      [HexString.ensure(collection).hex(), HexString.ensure(feeSchedule).hex(), price, amount, expiration_time],
    );
  }

  async cancelCollectionOffer(collectionOffer: MaybeHexString, coin: string = APTOS_COIN): Promise<TransactionPayload> {
    return this.buildTransactionPayload(COLLECTION_OFFER, "cancel", [coin], [HexString.ensure(collectionOffer).hex()]);
  }

  async fillCollectionOfferForTokenv1(
    collectionOffer: MaybeHexString,
    tokenName: string,
    propertyVersion: bigint,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COLLECTION_OFFER,
      "sell_tokenv1_entry",
      [coin],
      [HexString.ensure(collectionOffer).hex(), tokenName, propertyVersion.toString(10)],
    );
  }

  async fillCollectionOfferForTokenv2(
    collectionOffer: MaybeHexString,
    token: MaybeHexString,
    coin: string = APTOS_COIN,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      COLLECTION_OFFER,
      "sell_tokenv1",
      [coin],
      [HexString.ensure(collectionOffer).hex(), HexString.ensure(token).hex()],
    );
  }

  // Fee schedule operations

  async initFeeSchedule(
    feeAddress: MaybeHexString,
    biddingFee: bigint,
    listingFee: bigint,
    commissionDenominator: bigint,
    commissionNumerator: bigint,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      FEE_SCHEDULE,
      "init",
      [],
      [
        HexString.ensure(feeAddress).hex(),
        biddingFee.toString(10),
        listingFee.toString(10),
        commissionDenominator.toString(10),
        commissionNumerator.toString(10),
      ],
    );
  }

  async initEmptyFeeSchedule(feeAddress: MaybeHexString): Promise<TransactionPayload> {
    return this.buildTransactionPayload(FEE_SCHEDULE, "empty", [], [HexString.ensure(feeAddress).hex()]);
  }

  async setFeeAddress(feeSchedule: MaybeHexString, feeAddress: MaybeHexString): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      FEE_SCHEDULE,
      "set_fee_address",
      [],
      [HexString.ensure(feeSchedule).hex(), HexString.ensure(feeAddress).hex()],
    );
  }

  async setFixedRateListingFee(feeSchedule: MaybeHexString, fee: bigint): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      FEE_SCHEDULE,
      "set_fixed_rate_listing_fee",
      [],
      [HexString.ensure(feeSchedule).hex(), fee.toString(10)],
    );
  }

  async setFixedRateBiddingFee(feeSchedule: MaybeHexString, fee: bigint): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      FEE_SCHEDULE,
      "set_fixed_rate_bidding_fee",
      [],
      [HexString.ensure(feeSchedule).hex(), fee.toString(10)],
    );
  }

  async setFixedRateCommission(feeSchedule: MaybeHexString, commission: bigint): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      FEE_SCHEDULE,
      "set_fixed_rate_commission",
      [],
      [HexString.ensure(feeSchedule).hex(), commission.toString(10)],
    );
  }

  async setPercentageRateCommission(
    feeSchedule: MaybeHexString,
    commissionDenominator: bigint,
    commissionNumerator: bigint,
  ): Promise<TransactionPayload> {
    return this.buildTransactionPayload(
      FEE_SCHEDULE,
      "set_percentage_rate_commission",
      [],
      [HexString.ensure(feeSchedule).hex(), commissionDenominator.toString(10), commissionNumerator.toString(10)],
    );
  }

  // View functions
  // TODO: Collection offer view functions
  // TODO: Coin listing view functions
  // TODO: Listing view functions

  async feeAddress(feeSchedule: MaybeHexString, ledgerVersion?: bigint): Promise<HexString> {
    let outputs = await this.view(FEE_SCHEDULE, "fee_address", [], [feeSchedule], ledgerVersion);

    return HexString.ensure(outputs[0].toString());
  }

  async listingFee(feeSchedule: MaybeHexString, ledgerVersion?: bigint): Promise<bigint> {
    let outputs = await this.view(FEE_SCHEDULE, "listing_fee", [], [feeSchedule, 0], ledgerVersion);

    return BigInt(outputs[0].toString());
  }

  async biddingFee(feeSchedule: MaybeHexString, ledgerVersion?: bigint): Promise<bigint> {
    let outputs = await this.view(FEE_SCHEDULE, "bidding_fee", [], [feeSchedule, 0], ledgerVersion);

    return BigInt(outputs[0].toString());
  }

  async commission(feeSchedule: MaybeHexString, price: bigint, ledgerVersion?: bigint): Promise<bigint> {
    let outputs = await this.view(FEE_SCHEDULE, "commission", [], [feeSchedule, price.toString(10)], ledgerVersion);

    return BigInt(outputs[0].toString());
  }

  // Helpers

  async view(module: string, func: string, typeArguments: string[], args: any[], ledgerVersion?: bigint) {
    return await this.provider.view(
      {
        function: `${this.code_location}::${module}::${func}`,
        type_arguments: typeArguments,
        arguments: args,
      },
      ledgerVersion?.toString(10),
    );
  }

  buildTransactionPayload(module: string, func: string, type: string[], args: any[]): TransactionPayload {
    return {
      type: "entry_function_payload",
      function: func,
      type_arguments: type,
      arguments: args,
    };
  }

  /**
   * Submits a transaction generated from one of the above functions
   *
   * @param sender
   * @param payload
   * @param extraArgs
   */
  async submitTransaction(
    sender: AptosAccount,
    payload: TransactionPayload,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    const builder = new TransactionBuilderRemoteABI(this.provider, {
      sender: sender.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(payload.function, payload.type_arguments, payload.arguments);

    const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTxn);
    const pendingTransaction = await this.provider.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }
}
