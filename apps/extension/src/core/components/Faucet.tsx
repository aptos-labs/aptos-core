// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
} from '@chakra-ui/react';
import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import { useNetworks } from 'core/hooks/useNetworks';
import { useActiveAccount } from 'core/hooks/useAccounts';
import useFundAccount from 'core/mutations/faucet';
import { DefaultNetworks } from 'shared/types';
import { useAnalytics } from 'core/hooks/useAnalytics';
import { faucetEvents } from 'core/utils/analytics/events';
import { aptosCoinStructTag } from 'core/constants';
import { toast } from './Toast';

const defaultFundAmount = 50000;

export default function Faucet() {
  const { activeAccountAddress } = useActiveAccount();
  const { activeNetwork, activeNetworkName } = useNetworks();
  const { fundAccount, isFunding } = useFundAccount();
  const { trackEvent } = useAnalytics();

  const onClick = async () => {
    try {
      if (!fundAccount) {
        return;
      }

      await fundAccount({ address: activeAccountAddress, amount: defaultFundAmount });
      trackEvent({
        eventType: faucetEvents.RECEIVE_FAUCET,
        params: {
          amount: defaultFundAmount,
          coinType: aptosCoinStructTag,
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
