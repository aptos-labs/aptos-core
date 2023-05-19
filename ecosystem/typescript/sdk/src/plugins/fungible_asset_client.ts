import { AptosAccount } from "../account";
import { RawTransaction } from "../aptos_types";
import * as Gen from "../generated/index";
import { OptionalTransactionArgs, Provider } from "../providers";
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
   * Transfer an asset amount from the sender primary_store to the recipient primary_store
   *
   * Use this method to transfer any fungible asset including fungible token.
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
    const rawTxn = await this.generateTransferAmount(sender, assetAddress, recipient, amount, assetType, extraArgs);
    return await this.submit(sender, rawTxn);
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

  /**
   *
   * Generate a transfer transaction.
   * This method can be used if you want/need to get the raw transaction so you can
   * first simulate the transaction and then sign and submit it.
   *
   * @param sender The sender account
   * @param assetAddress The fungible asset address - For example if you’re transferring USDT, this would be the USDT address
   * @param recipient Recipient address
   * @param amount Number of assets to transfer
   * @param assetType The fungible asset type
   * @returns Raw Transaction
   */
  async generateTransferAmount(
    sender: AptosAccount,
    assetAddress: MaybeHexString,
    recipient: MaybeHexString,
    amount: number | bigint,
    assetType?: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<RawTransaction> {
    const builder = new TransactionBuilderRemoteABI(this.provider, {
      sender: sender.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x1::primary_fungible_store::transfer",
      [assetType || this.assetType],
      [HexString.ensure(assetAddress).hex(), HexString.ensure(recipient).hex(), amount],
    );
    return rawTxn;
  }

  /**
   * Submit a transaction to chain
   *
   * @param sender The sender account
   * @param rawTransaction A generated raw transaction
   * @returns The hash of the transaction submitted to the API
   */
  async submit(sender: AptosAccount, rawTransaction: RawTransaction): Promise<string> {
    return await this.provider.signAndSubmitTransaction(sender, rawTransaction);
  }
}
