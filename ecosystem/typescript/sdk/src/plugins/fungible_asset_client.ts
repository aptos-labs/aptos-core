import { AptosAccount } from "../account";
import { RawTransaction } from "../aptos_types";
import * as Gen from "../generated/index";
import { OptionalTransactionArgs, Provider } from "../providers";
import { TransactionBuilderRemoteABI } from "../transaction_builder";
import { MaybeHexString, HexString } from "../utils";
import { AnyNumber } from "../bcs";

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
   *  Transfer `amount` of fungible asset from sender's primary store to recipient's primary store.
   *
   * Use this method to transfer any fungible asset including fungible token.
   *
   * @param sender The sender account
   * @param fungibleAssetMetadataAddress The fungible asset address.
   * For example if you’re transferring USDT this would be the USDT address
   * @param recipient Recipient address
   * @param amount Number of assets to transfer
   * @returns The hash of the transaction submitted to the API
   */
  async transfer(
    sender: AptosAccount,
    fungibleAssetMetadataAddress: MaybeHexString,
    recipient: MaybeHexString,
    amount: number | bigint,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<string> {
    const rawTransaction = await this.generateTransfer(
      sender,
      fungibleAssetMetadataAddress,
      recipient,
      amount,
      extraArgs,
    );
    const txnHash = await this.provider.signAndSubmitTransaction(sender, rawTransaction);
    return txnHash;
  }

  /**
   * Get the balance of a fungible asset from the account's primary fungible store.
   *
   * @param account Account that you want to get the balance of.
   * @param fungibleAssetMetadataAddress The fungible asset address you want to check the balance of
   * @returns Promise that resolves to the balance
   */
  async getPrimaryBalance(account: MaybeHexString, fungibleAssetMetadataAddress: MaybeHexString): Promise<bigint> {
    const payload: Gen.ViewRequest = {
      function: "0x1::primary_fungible_store::balance",
      type_arguments: [this.assetType],
      arguments: [HexString.ensure(account).hex(), HexString.ensure(fungibleAssetMetadataAddress).hex()],
    };
    const response = await this.provider.view(payload);
    return BigInt((response as any)[0]);
  }

  /**
   *
   * Generate a transfer transaction that can be used to sign and submit to transfer an asset amount
   * from the sender primary fungible store to the recipient primary fungible store.
   *
   * This method can be used if you want/need to get the raw transaction so you can
   * first simulate the transaction and then sign and submit it.
   *
   * @param sender The sender account
   * @param fungibleAssetMetadataAddress The fungible asset address.
   * For example if you’re transferring USDT this would be the USDT address
   * @param recipient Recipient address
   * @param amount Number of assets to transfer
   * @returns Raw Transaction
   */
  async generateTransfer(
    sender: AptosAccount,
    fungibleAssetMetadataAddress: MaybeHexString,
    recipient: MaybeHexString,
    amount: AnyNumber,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<RawTransaction> {
    const builder = new TransactionBuilderRemoteABI(this.provider, {
      sender: sender.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x1::primary_fungible_store::transfer",
      [this.assetType],
      [HexString.ensure(fungibleAssetMetadataAddress).hex(), HexString.ensure(recipient).hex(), amount],
    );
    return rawTxn;
  }
}
