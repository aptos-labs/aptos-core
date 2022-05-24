// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
} from '@chakra-ui/react';
import React from 'react';
import { FaFaucet } from 'react-icons/fa';
import useWalletState from 'core/hooks/useWalletState';
import { fundAccountWithFaucet } from 'core/queries/faucet';
import { useMutation, useQueryClient } from 'react-query';

export default function Faucet() {
  const { aptosAccount } = useWalletState();
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
    if (address) {
      await fundWithFaucet({ address });
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
