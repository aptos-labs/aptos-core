// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useNetworks } from 'core/hooks/useNetworks';
import { useMutation, useQueryClient } from 'react-query';
import { MaybeHexString } from 'aptos';
import queryKeys from 'core/queries/queryKeys';

interface UseFundAccountParams {
  address: MaybeHexString,
  amount: number,
}

export function useFundAccount() {
  const { faucetClient } = useNetworks();
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
    onSuccess: async (result, { address }: UseFundAccountParams) => {
      if (result) {
        await queryClient.invalidateQueries([
          queryKeys.getAccountCoinBalance,
          address,
        ]);
      }
    },
  });
  return {
    fundAccount: fundAccount ? mutateAsync : undefined,
    isFunding: isLoading,
    ...other,
  };
}

export default useFundAccount;
