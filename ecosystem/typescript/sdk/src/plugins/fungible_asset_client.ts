import { AptosAccount } from "../account";
import * as Gen from "../generated/index";
import { OptionalTransactionArgs, AptosClient, Provider } from "../providers";
import { TransactionBuilderRemoteABI } from "../transaction_builder";
import { MaybeHexString, HexString } from "../utils";

export class FungibleAssetClient {
  provider: Provider;
  readonly assetType: string = "0x1::fungible_asset::Metadata";

  /**
   * Creates new FungibleAssetClient instance
   *
   * @param provider Provider instance
   */
  constructor(provider: Provider) {
    this.provider = provider;
  }

  /**
   * Transfer an amount of fungible asset from the sender primary_fungible_store to the recipient primary_fungible_store
   *
   * Use this method to transfer any fungible asset.
   *
   * @param sender The sender account
   * @param assetAddress The fungible asset address - For example if you’re transferring USDT, this would be the USDT address
   * @param recipient Recipient address
   * @param amount Number of assets to transfer
   * @param assetType The fungible asset type
   * @returns The hash of the transaction submitted to the API
   */
  async transferAmount(
    sender: AptosAccount,
    assetAddress: MaybeHexString,
    recipient: MaybeHexString,
    amount: number | bigint,
    assetType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: sender.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x1::primary_fungible_store::transfer",
      [assetType || this.assetType],
      [HexString.ensure(assetAddress).hex(), HexString.ensure(recipient).hex(), amount],
    );
    const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }

  /**
   * Get the balance of an account's fungible asset.
   *
   * @param account Account that you want to get the balance of.
   * @param assetAddress The fungible asset address you want to check the balance of
   * @param assetType The fungible asset type
   * @returns Promise that resolves to the balance
   */
  async balance(account: MaybeHexString, assetAddress: MaybeHexString, assetType?: string): Promise<bigint> {
    const payload: Gen.ViewRequest = {
      function: "0x1::primary_fungible_store::balance",
      type_arguments: [assetType || this.assetType],
      arguments: [HexString.ensure(account).hex(), HexString.ensure(assetAddress).hex()],
    };
    const response = await this.provider.view(payload);
    return BigInt((response as any)[0]);
  }
}
