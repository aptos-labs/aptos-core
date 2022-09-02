// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Types } from 'aptos';
import { useTransaction } from 'core/queries/transaction';
import numeral from 'numeral';

export const formatCoinName = (coinName: string | undefined) => {
  switch (coinName) {
    case undefined:
    case 'AptosCoin':
      return 'APT';
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
  const amount = numeral(Number(payload.arguments[1])).format('0,0');
  const defaultCoinName = payload.type_arguments[0]?.split('::').pop();
  const coinName = formatCoinName(defaultCoinName);
  const gasUsed = numeral(txn.gas_used).format('0,0');

  return {
    amount,
    coinName,
    fullDatetime,
    gasUsed,
    recipient,
    ...txn,
  };
}
