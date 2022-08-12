// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import useGlobalStateContext from 'core/hooks/useGlobalState';
import { useMutation, useQueryClient } from 'react-query';
import { MaybeHexString } from 'aptos';
import queryKeys from 'core/queries/queryKeys';

interface UseFundAccountParams {
  address: MaybeHexString,
  amount: number,
}

export function useFundAccount() {
  const { faucetClient } = useGlobalStateContext();
  const queryClient = useQueryClient();

  const fundAccount = faucetClient
    ? ({ address, amount }: UseFundAccountParams) => faucetClient.fundAccount(address, amount)
    : undefined;

  const {
    isLoading,
    mutateAsync,
    ...other
  } = useMutation({
    mutationFn: fundAccount,
    onSuccess: async (data, { address }: UseFundAccountParams) => {
      await queryClient.invalidateQueries([
        queryKeys.getAccountCoinBalance,
        address,
      ]);
    },
  });
  return { fundAccount: mutateAsync, isFunding: isLoading, ...other };
}

export default useFundAccount;
