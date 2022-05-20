/* eslint-disable @typescript-eslint/no-unused-vars */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
  HStack,
  VStack,
} from '@chakra-ui/react';
import React, { useCallback } from 'react';
import { FaFaucet } from 'react-icons/fa';
import useWalletState from 'core/hooks/useWalletState';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import WalletLayout from 'core/layouts/WalletLayout';
import { fundAccountWithFaucet } from 'core/queries/faucet';
import { useMutation, useQueryClient } from 'react-query';
import WalletAccountBalance from 'core/components/WalletAccountBalance';
import TransferDrawer from 'core/components/TransferDrawer';

function Wallet() {
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

  const toAddressOnChange = useCallback(async (event: {
    target: any;
    type?: any;
  }) => {
    // toAddressOnChange(event);
    queryClient.invalidateQueries('getToAddressAccountExists');
  }, []);

  return (
    <WalletLayout>
      <VStack width="100%" paddingTop={8}>
        <WalletAccountBalance />
        <HStack spacing={4}>
          <Button
            isLoading={isFaucetLoading}
            leftIcon={<FaFaucet />}
            onClick={faucetOnClick}
            isDisabled={isFaucetLoading}
          >
            Faucet
          </Button>
          <TransferDrawer />
        </HStack>
      </VStack>
    </WalletLayout>
  );
}

export default withSimulatedExtensionContainer(Wallet);
