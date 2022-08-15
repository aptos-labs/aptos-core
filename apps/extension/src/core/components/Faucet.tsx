// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
} from '@chakra-ui/react';
import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import { aptosCoinStructTag } from 'core/constants';
import Analytics from 'core/utils/analytics/analytics';
import { faucetEvents } from 'core/utils/analytics/events';
import useGlobalStateContext, { DefaultNetworks } from 'core/hooks/useGlobalState';
import useFundAccount from 'core/mutations/faucet';
import { NodeUrl } from 'core/utils/network';
import { toast } from './Toast';

const defaultFundAmount = 50000;

export default function Faucet() {
  const {
    activeAccountAddress,
    activeNetwork,
    activeNetworkName,
  } = useGlobalStateContext();
  const { fundAccount, isFunding } = useFundAccount();

  const onClick = async () => {
    try {
      await fundAccount({ address: activeAccountAddress!, amount: defaultFundAmount });
      Analytics.event({
        eventType: faucetEvents.RECEIVE_FAUCET,
        params: {
          address: activeAccountAddress,
          amount: defaultFundAmount,
          coinType: aptosCoinStructTag,
          network: activeNetwork?.nodeUrl as NodeUrl,
        },
      });
    } catch (err) {
      const localhostMessage = (activeNetworkName === DefaultNetworks.Localhost)
        ? 'If you are on localhost, please ensure that the faucet is running.'
        : undefined;
      toast({
        description: `Error accessing faucet at ${activeNetwork?.faucetUrl}. ${localhostMessage}`,
        status: 'error',
        title: 'Error calling faucet',
      });
    }
  };

  return (
    <Button
      isLoading={isFunding}
      leftIcon={<FontAwesomeIcon icon={faFaucet} />}
      onClick={onClick}
      isDisabled={isFunding}
      colorScheme="teal"
      variant="outline"
    >
      Faucet
    </Button>
  );
}
