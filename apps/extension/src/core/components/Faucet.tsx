// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button, useToast,
} from '@chakra-ui/react';
import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import useWalletState from 'core/hooks/useWalletState';
import { fundAccountWithFaucet } from 'core/queries/faucet';
import { useMutation, useQueryClient } from 'react-query';
import { LOCAL_FAUCET_URL } from 'core/constants';
import Analytics from 'core/utils/analytics/analytics';
import { faucetEvents } from 'core/utils/analytics/events';

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
      Analytics.event({
        eventType: faucetEvents.RECEIVE_FAUCET,
        params: {
          address: aptosAccount?.address().hex(),
          amount: 5000,
          coinType: '0x1::TestCoin::TestCoin',
          network: aptosNetwork,
        },
      });
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
      leftIcon={<FontAwesomeIcon icon={faFaucet} />}
      onClick={faucetOnClick}
      isDisabled={isFaucetLoading}
    >
      Faucet
    </Button>
  );
}
