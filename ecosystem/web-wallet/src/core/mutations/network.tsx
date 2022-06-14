// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useToast } from '@chakra-ui/react';
import { FaucetClient } from 'aptos';
import { LOCAL_NODE_URL } from 'core/constants';
import useWalletState from 'core/hooks/useWalletState';
import { getAccountExists } from 'core/queries/account';
import { AptosNetwork, getFaucetNetworkFromAptosNetwork, networkUriMap } from 'core/utils/network';
import { useMutation, useQueryClient } from 'react-query';

interface UseSwitchNetworkMutationProps {
  event: AptosNetwork;
  localTestnetIsLive: boolean | undefined;
}

export const useSwitchNetwork = () => {
  const queryClient = useQueryClient();
  const {
    aptosAccount, aptosNetwork, updateNetworkState, updateWalletState,
  } = useWalletState();
  const toast = useToast();

  const mutation = async ({
    event,
    localTestnetIsLive,
  }: UseSwitchNetworkMutationProps): Promise<void> => {
    const newAptosNetwork = event;
    const newFaucetNetwork = getFaucetNetworkFromAptosNetwork(newAptosNetwork);
    // switching to local testnet
    if (newAptosNetwork === LOCAL_NODE_URL && !localTestnetIsLive) {
      return;
    }
    const accountExists = await getAccountExists({
      address: aptosAccount?.address().hex(),
      nodeUrl: newAptosNetwork,
    });
    if (!accountExists && aptosAccount) {
      const faucetClient = new FaucetClient(newAptosNetwork, newFaucetNetwork);
      try {
        await faucetClient.fundAccount(aptosAccount.address(), 0);
        updateWalletState({ aptosAccountState: aptosAccount });
        toast({
          description: `No account with your credentials existed on ${networkUriMap[newAptosNetwork]}, a new account was initialized`,
          duration: 5000,
          isClosable: true,
          status: 'success',
          title: `Created new account on ${networkUriMap[newAptosNetwork]}`,
          variant: 'solid',
        });
      } catch (err) {
        toast({
          description: `Unable to access ${newFaucetNetwork}, you are still on ${aptosNetwork}`,
          duration: 5000,
          isClosable: true,
          status: 'error',
          title: 'Error accessing faucet',
          variant: 'solid',
        });
        throw new Error(`Unable to access ${newFaucetNetwork}, you are still on ${aptosNetwork}`);
      }
    }
    updateNetworkState(newAptosNetwork);
    queryClient.invalidateQueries('getAccountResources');
  };

  return useMutation(mutation);
};

export default useSwitchNetwork;
