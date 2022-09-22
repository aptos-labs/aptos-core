import { Types } from 'aptos';
import { MoveAbortDetails } from 'shared/move/abort';
import { CoinInfoData } from './resource';

export interface CoinBalanceChange {
  amount: bigint,
  coinInfo?: CoinInfoData,
}

export type CoinBalanceChangesByCoinType = Record<string, CoinBalanceChange>;
export type CoinBalanceChangesByAccount = Record<string, CoinBalanceChangesByCoinType>;

export interface BaseTransaction {
  coinBalanceChanges: CoinBalanceChangesByAccount,
  error?: MoveAbortDetails,
  expirationTimestamp: number,
  gasFee: number,
  gasUnitPrice: number,
  hash: string,
  payload: Types.EntryFunctionPayload,
  rawChanges: Types.WriteSetChange[],
  success: boolean,
  timestamp: number,
  version: number
}

export type CoinTransferTransaction = BaseTransaction & {
  amount: bigint,
  coinInfo?: CoinInfoData,
  coinType: string,
  recipient: string,
  sender: string,
  type: 'transfer',
};

export type CoinMintTransaction = BaseTransaction & {
  amount: bigint,
  coinInfo?: CoinInfoData,
  recipient: string,
  type: 'mint',
};

export type GenericTransaction = BaseTransaction & {
  sender: string,
  type: 'generic',
};

export type Transaction = CoinTransferTransaction
| CoinMintTransaction
| GenericTransaction;
