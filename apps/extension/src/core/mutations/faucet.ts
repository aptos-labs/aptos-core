// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useNetworks } from 'core/hooks/useNetworks';
import { useMutation, useQueryClient } from 'react-query';
import { MaybeHexString } from 'aptos';
import queryKeys from 'core/queries/queryKeys';
import { useAnalytics } from 'core/hooks/useAnalytics';
import { faucetEvents } from 'core/utils/analytics/events';
import { aptosCoinStructTag, defaultFundAmount } from 'core/constants';
import { faucetOnErrorToast } from 'core/components/Toast';

interface UseFundAccountParams {
  address: MaybeHexString,
  amount: number,
}

export function useFundAccount() {
  const { activeNetwork, faucetClient } = useNetworks();
  const queryClient = useQueryClient();
  const { trackEvent } = useAnalytics();

  const fundAccount = faucetClient
    ? ({ address, amount }: UseFundAccountParams) => faucetClient.fundAccount(address, amount)
    : undefined;

  const {
    isLoading,
    mutateAsync,
    ...other
  } = useMutation({
    mutationFn: fundAccount,
    onError: (err: any) => {
      trackEvent({
        eventType: faucetEvents.ERROR_RECEIVE_FAUCET,
        params: {
          amount: defaultFundAmount,
          coinType: aptosCoinStructTag,
          error: String(err),
        },
      });
      faucetOnErrorToast(activeNetwork, err?.body);
    },
    onSuccess: async (result, { address }: UseFundAccountParams) => {
      if (result) {
        trackEvent({
          eventType: faucetEvents.RECEIVE_FAUCET,
          params: {
            amount: defaultFundAmount,
            coinType: aptosCoinStructTag,
          },
        });
        await Promise.all([
          queryClient.invalidateQueries([
            queryKeys.getAccountOctaCoinBalance,
            address,
          ]),
          queryClient.invalidateQueries([
            queryKeys.getAccountCoinResources,
            address,
          ]),
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
