import { TxnBuilderTypes, TransactionBuilder } from 'aptos';

export interface TransactionConfig {
  address: string,
  chainId: number,
  expSecFromNow?: number,
  gasAmount?: number | bigint,
  gasUnitPrice?: number | bigint,
  sequenceNumber: number | bigint,
}

export function createRawTransaction(
  transactionPayload: TxnBuilderTypes.TransactionPayload,
  config: TransactionConfig,
): TxnBuilderTypes.RawTransaction {
  const {
    address, chainId, expSecFromNow, gasAmount, gasUnitPrice, sequenceNumber,
  } = {
    expSecFromNow: Math.floor(Date.now() / 1000) + 20,
    gasAmount: 1000,
    gasUnitPrice: 1,
    ...config,
  };

  return new TxnBuilderTypes.RawTransaction(
    TxnBuilderTypes.AccountAddress.fromHex(address),
    BigInt(sequenceNumber),
    transactionPayload,
    BigInt(gasAmount), // TODO: wallet needs to ask users for how much gas to use here
    BigInt(gasUnitPrice), // Unit gas price
    BigInt(expSecFromNow), // Transactions are valid for 20s before expires
    new TxnBuilderTypes.ChainId(Number(chainId)),
  );
}

export function getSigningMessage(rawTransaction: TxnBuilderTypes.RawTransaction) {
  return TransactionBuilder.getSigningMessage(rawTransaction).toString('hex');
}
