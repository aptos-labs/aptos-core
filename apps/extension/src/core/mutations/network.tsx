// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { FaucetClient } from 'aptos';
import toast from 'core/components/Toast';
import { useWalletState } from 'core/hooks/useWalletState';
import { getAccountExists } from 'core/queries/account';
import queryKeys from 'core/queries/queryKeys';
import {
  getFaucetUrlFromNodeUrl, NodeUrl, nodeUrlMap, nodeUrlReverseMap,
} from 'core/utils/network';
import { useMutation, useQueryClient } from 'react-query';

interface UseSwitchNetworkMutationProps {
  event: NodeUrl;
  localTestnetIsLive: boolean | undefined;
}

export const useSwitchNetwork = () => {
  const queryClient = useQueryClient();
  const {
    aptosAccount, nodeUrl, updateNetworkState,
  } = useWalletState();

  const mutation = async ({
    event,
    localTestnetIsLive,
  }: UseSwitchNetworkMutationProps): Promise<void> => {
    const newNodeUrl = event;
    const newFaucetNetwork = getFaucetUrlFromNodeUrl(newNodeUrl);
    // switching to local testnet
    if (newNodeUrl === nodeUrlMap.Localhost && !localTestnetIsLive) {
      return;
    }
    const accountExists = await getAccountExists({
      address: aptosAccount?.address().hex(),
      nodeUrl: newNodeUrl,
    });
    if (!accountExists && aptosAccount && newFaucetNetwork) {
      const faucetClient = new FaucetClient(newNodeUrl, newFaucetNetwork);
      try {
        await faucetClient.fundAccount(aptosAccount.address(), 0);
        toast({
          description: `No account with your credentials existed on ${nodeUrlReverseMap[newNodeUrl]}, a new account was initialized`,
          status: 'success',
          title: `Created new account on ${nodeUrlReverseMap[newNodeUrl]}`,
        });
      } catch (err) {
        toast({
          description: `Unable to access ${newFaucetNetwork}, you are still on ${nodeUrl}`,
          status: 'error',
          title: 'Error accessing faucet',
        });
        throw new Error(`Unable to access ${newFaucetNetwork}, you are still on ${nodeUrl}`);
      }
    }
    updateNetworkState(newNodeUrl);
    queryClient.invalidateQueries(queryKeys.getAccountCoinBalance);
  };

  return useMutation(mutation);
};

export default useSwitchNetwork;
