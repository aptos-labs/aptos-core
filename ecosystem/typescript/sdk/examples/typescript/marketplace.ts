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
    type: string,
    function: string,
    type_arguments: string[],
    arguments: any[]
};

const APTOS_COIN: string = "0x1::aptos_coin::AptosCoin"
const FEE_SCHEDULE: string = "fee_schedule";
const COIN_LISTING: string = "coin_listing";

/**
 * Class for managing aptos_token
 */
export class Marketplace {
    readonly provider: Provider;
    readonly code_location: HexString;

    constructor(provider: Provider, code_location: MaybeHexString) {
        this.provider = provider;
        this.code_location = HexString.ensure(code_location);
    }

    // TODO: Collection offer operations
    // TODO: Listing Token V1 extraction


    // Coin listing operations
    public async buildInitFixedPrice(
        seller: MaybeHexString,
        object: MaybeHexString,
        feeAddress: MaybeHexString,
        startTime: bigint,
        price: bigint,
        coin: string = APTOS_COIN,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            COIN_LISTING,
            "init_fixed_price",
            [coin],
            [
                HexString.ensure(object).hex(),
                HexString.ensure(feeAddress).hex(),
                startTime.toString(10),
                price.toString(10)
            ],
        );
    }

    public async initAuction(
        object: MaybeHexString,
        feeAddress: MaybeHexString, // Address for fees to be sent to
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
                HexString.ensure(feeAddress).hex(),
                startTime.toString(10),
                startingBid.toString(10),
                bidIncrement.toString(10),
                auctionEndTime.toString(10),
                minimumBidTimeBeforeEnd.toString(10),
                buyItNowPrice?.toString(10),
            ],
        );
    }

    public async initFixedPriceForTokenv1(
        tokenCreator: MaybeHexString,
        tokenCollection: string,
        tokenName: string,
        tokenPropertyVersion: bigint,
        feeAddress: MaybeHexString, // Address for fees to be sent to
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
                HexString.ensure(feeAddress).hex(),
                startTime.toString(10),
                price.toString(10)
            ],
        );
    }

    public async initAuctionForTokenv1(
        tokenCreator: MaybeHexString,
        tokenCollection: string,
        tokenName: string,
        feeAddress: MaybeHexString, // Address for fees to be sent to
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
                HexString.ensure(feeAddress).hex(),
                startTime.toString(10),
                startingBid.toString(10),
                bidIncrement.toString(10),
                auctionEndTime.toString(10),
                minimumBidTimeBeforeEnd.toString(10),
                buyItNowPrice?.toString(10),
            ],
        );
    }

    public async purchase(
        listing: MaybeHexString,
        coin: string = APTOS_COIN,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            COIN_LISTING,
            "purchase",
            [coin],
            [
                HexString.ensure(listing).hex(),
            ],
        );
    }

    public async end_fixed_price(
        listing: MaybeHexString,
        coin: string = APTOS_COIN,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            COIN_LISTING,
            "end_fixed_price",
            [coin],
            [
                HexString.ensure(listing).hex(),
            ],
        );
    }

    public async bid(
        bidder: MaybeHexString,
        listing: MaybeHexString,
        bid_amount: bigint,
        coin: string = APTOS_COIN,
        extraArgs?: OptionalTransactionArgs,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            COIN_LISTING,
            "bid",
            [coin],
            [
                HexString.ensure(listing).hex(),
                bid_amount.toString(10)
            ],
        );
    }

    public async complete_auction(
        listing: MaybeHexString,
        bid_amount: bigint,
        coin: string = APTOS_COIN,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            COIN_LISTING,
            "complete_auction",
            [coin],
            [
                HexString.ensure(listing).hex(),
            ],
        );
    }


    // Admin operations

    public async initFeeSchedule(
        feeAddress: MaybeHexString, // Address for fees to be sent to
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

    public async initEmptyFeeSchedule(
        feeAddress: MaybeHexString, // Address for fees to be sent to
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            FEE_SCHEDULE,
            "empty",
            [],
            [
                HexString.ensure(feeAddress).hex(),
            ],
        );
    }

    public async setFeeAddress(
        feeSchedule: MaybeHexString, // Address of marketplace
        feeAddress: MaybeHexString, // Address for fees to be sent to
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            FEE_SCHEDULE,
            "set_fee_address",
            [],
            [
                HexString.ensure(feeSchedule).hex(),
                HexString.ensure(feeAddress).hex(),
            ],
        );
    }

    public async setFixedRateListingFee(
        feeSchedule: MaybeHexString, // Address of marketplace
        fee: bigint,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            FEE_SCHEDULE,
            "set_fixed_rate_listing_fee",
            [],
            [
                HexString.ensure(feeSchedule).hex(),
                fee.toString(10),
            ],
        );
    }

    public async setFixedRateBiddingFee(
        feeSchedule: MaybeHexString, // Address of marketplace
        fee: bigint,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            FEE_SCHEDULE,
            "set_fixed_rate_bidding_fee",
            [],
            [
                HexString.ensure(feeSchedule).hex(),
                fee.toString(10),
            ],
        );
    }

    public async setFixedRateCommission(
        feeSchedule: MaybeHexString, // Address of marketplace
        commission: bigint,
        extraArgs?: OptionalTransactionArgs,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            FEE_SCHEDULE,
            "set_fixed_rate_commission",
            [],
            [
                HexString.ensure(feeSchedule).hex(),
                commission.toString(10)
            ],
        );
    }

    public async setPercentageRateCommission(
        feeSchedule: MaybeHexString, // Address of marketplace
        commissionDenominator: bigint,
        commissionNumerator: bigint,
    ): Promise<TransactionPayload> {
        return this.buildTransactionPayload(
            FEE_SCHEDULE,
            "set_percentage_rate_commission",
            [],
            [
                HexString.ensure(feeSchedule).hex(),
                commissionDenominator.toString(10),
                commissionNumerator.toString(10)
            ],
        );
    }

    // View functions
    // TODO: Coin listing view functions
    // TODO: Listing view functions

    public async feeAddress(
        feeSchedule: MaybeHexString, // Address of marketplace
        ledgerVersion?: bigint,
    ): Promise<HexString> {
        let outputs = await this.view(
            FEE_SCHEDULE,
            "fee_address",
            [],
            [
                feeSchedule,
            ],
            ledgerVersion
        )

        return HexString.ensure(outputs[0].toString());
    }

    public async listingFee(
        feeSchedule: MaybeHexString, // Address of marketplace
        ledgerVersion?: bigint,
    ): Promise<bigint> {
        let outputs = await this.view(
            FEE_SCHEDULE,
            "listing_fee",
            [],
            [
                feeSchedule,
                0
            ],
            ledgerVersion
        )

        return BigInt(outputs[0].toString());
    }

    public async biddingFee(
        feeSchedule: MaybeHexString, // Address of marketplace
        ledgerVersion?: bigint,
    ): Promise<bigint> {
        let outputs = await this.view(
            FEE_SCHEDULE,
            "bidding_fee",
            [],
            [
                feeSchedule,
                0
            ],
            ledgerVersion
        )

        return BigInt(outputs[0].toString());
    }

    public async commission(
        feeSchedule: MaybeHexString, // Address of marketplace
        price: bigint,
        ledgerVersion?: bigint,
    ): Promise<bigint> {
        let outputs = await this.view(
            FEE_SCHEDULE,
            "commission",
            [],
            [
                feeSchedule,
                price.toString(10)
            ],
            ledgerVersion
        )

        return BigInt(outputs[0].toString());
    }

    // Helpers

    async view(
        module: string,
        func: string,
        typeArguments: string[],
        args: any[],
        ledgerVersion?: bigint
    ) {
        return await this.provider.view(
            {
                function: `${this.code_location}::${module}::${func}`,
                type_arguments: typeArguments,
                arguments: args,
            },
            ledgerVersion?.toString(10)
        );
    }

    buildTransactionPayload(
        module: string,
        func: string,
        type: string[],
        args: any[],
    ): TransactionPayload {
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
    public async submitTransaction(
        sender: AptosAccount,
        payload: TransactionPayload,
        extraArgs?: OptionalTransactionArgs,
    ): Promise<string> {
        const builder = new TransactionBuilderRemoteABI(this.provider, {
            sender: sender.address(),
            ...extraArgs,
        });
        const rawTxn = await builder.build(
            payload.function, payload.type_arguments, payload.arguments
        );

        const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTxn);
        const pendingTransaction = await this.provider.submitSignedBCSTransaction(bcsTxn);
        return pendingTransaction.hash;
    }
}