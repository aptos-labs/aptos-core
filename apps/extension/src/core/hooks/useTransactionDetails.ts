// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';
import { useTransaction } from 'core/queries/transaction';
import { APTOS_UNIT, formatCoin } from 'core/utils/coin';
import numeral from 'numeral';

export const formatCoinName = (coinName: string | undefined) => {
  switch (coinName) {
    case undefined:
    case 'AptosCoin':
      return APTOS_UNIT;
    default:
      return coinName;
  }
};

export default function useTransactionDetails(version?: number) {
  const { data: txn } = useTransaction(version);
  if (!txn) {
    return null;
  }

  const datetime = new Date(Number(txn.timestamp) / 1000);
  const fullDatetime = datetime.toLocaleDateString('en-us', {
    day: 'numeric',
    hour: 'numeric',
    minute: 'numeric',
    month: 'short',
    year: 'numeric',
  });

  const payload = txn.payload as Types.EntryFunctionPayload;
  const recipient = payload.arguments[0] as string;
  const defaultCoinName = payload.type_arguments[0]?.split('::').pop();
  const coinName = formatCoinName(defaultCoinName);
  const amount = (coinName === APTOS_UNIT)
    ? formatCoin(Number(payload.arguments[1]), { decimals: 8 })
    : `${numeral(Number(payload.arguments[1])).format('0,0')} ${coinName}`;
  const gasUsed = formatCoin(Number(txn.gas_used), { decimals: 8 });

  return {
    amount,
    coinName,
    fullDatetime,
    gasUsed,
    recipient,
    ...txn,
  };
}
