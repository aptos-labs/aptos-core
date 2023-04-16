import { AptosAccount } from "../account";
import { OptionalTransactionArgs, AptosClient, Provider } from "../providers";
import { TransactionBuilderRemoteABI } from "../transaction_builder";
import { MaybeHexString, HexString } from "../utils";

export class FungibleAsset {
  provider: Provider;

  /**
   * Creates new FungibleAsset instance
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
   * @param token The asset address - For example if you’re transferring USDT, this would be the USDT address
   * @param recipient Recipient primary wallet address
   * @param assetType The asset type
   * @returns The hash of the transaction submitted to the API
   */
  async transferAmount(
    sender: AptosAccount,
    assetAddress: MaybeHexString,
    recipient: MaybeHexString,
    amount: number = 0,
    assetType: string,
    extraArgs?: OptionalTransactionArgs,
  ) {
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: sender.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      "0x1::primary_store::transfer",
      [assetType],
      [HexString.ensure(assetAddress).hex(), HexString.ensure(recipient).hex(), amount],
    );
    const bcsTxn = AptosClient.generateBCSTransaction(sender, rawTxn);
    const pendingTransaction = await this.provider.aptosClient.submitSignedBCSTransaction(bcsTxn);
    return pendingTransaction.hash;
  }
}
