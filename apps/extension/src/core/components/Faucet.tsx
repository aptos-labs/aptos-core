// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
} from '@chakra-ui/react';
import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import useWalletState from 'core/hooks/useWalletState';
import { fundAccountWithFaucet } from 'core/queries/faucet';
import { useMutation, useQueryClient } from 'react-query';
import { aptosCoinStructTag, LOCAL_FAUCET_URL } from 'core/constants';
import Analytics from 'core/utils/analytics/analytics';
import { faucetEvents } from 'core/utils/analytics/events';
import queryKeys from 'core/queries/queryKeys';
import { toast } from './Toast';

export default function Faucet() {
  const { aptosAccount, aptosNetwork, faucetNetwork } = useWalletState();
  const queryClient = useQueryClient();
  const {
    isLoading: isFaucetLoading,
    mutateAsync: fundWithFaucet,
  } = useMutation(fundAccountWithFaucet, {
    onSettled: (_data, error) => {
      if (error) {
        toast({
          description: `Error accessing faucet at ${faucetNetwork}: ${error}`,
          status: 'error',
          title: 'Faucet failure',
        });
      }
      queryClient.invalidateQueries(queryKeys.getAccountCoinBalance);
      Analytics.event({
        eventType: faucetEvents.RECEIVE_FAUCET,
        params: {
          address: aptosAccount?.address().hex(),
          amount: 5000,
          coinType: aptosCoinStructTag,
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
