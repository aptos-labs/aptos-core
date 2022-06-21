// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button, useToast,
} from '@chakra-ui/react';
import React from 'react';
import { FaFaucet } from 'react-icons/fa';
import useWalletState from 'core/hooks/useWalletState';
import { fundAccountWithFaucet } from 'core/queries/faucet';
import { useMutation, useQueryClient } from 'react-query';
import { LOCAL_FAUCET_URL } from 'core/constants';

export default function Faucet() {
  const { aptosAccount, aptosNetwork, faucetNetwork } = useWalletState();
  const toast = useToast();
  const queryClient = useQueryClient();
  const {
    isLoading: isFaucetLoading,
    mutateAsync: fundWithFaucet,
  } = useMutation(fundAccountWithFaucet, {
    onSettled: () => {
      queryClient.invalidateQueries('getAccountResources');
    },
  });

  const address = aptosAccount?.address().hex();

  const faucetOnClick = async () => {
    try {
      if (address) {
        await fundWithFaucet({ address, faucetUrl: faucetNetwork, nodeUrl: aptosNetwork });
      }
    } catch (err) {
      const localhostMessage = (faucetNetwork === LOCAL_FAUCET_URL)
        ? 'If you are on localhost, please ensure that the faucet is running.'
        : undefined;
      toast({
        description: `Error accessing faucet at ${faucetNetwork}. ${localhostMessage}`,
        duration: 5000,
        isClosable: true,
        status: 'error',
        title: 'Error calling faucet',
        variant: 'solid',
      });
    }
  };

  return (
    <Button
      isLoading={isFaucetLoading}
      leftIcon={<FaFaucet />}
      onClick={faucetOnClick}
      isDisabled={isFaucetLoading}
    >
      Faucet
    </Button>
  );
}
