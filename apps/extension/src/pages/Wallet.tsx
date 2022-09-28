/* eslint-disable @typescript-eslint/no-unused-vars */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Alert,
  AlertDescription,
  AlertIcon,
  Flex,
  HStack,
  VStack,
  Text,
  SimpleGrid,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import WalletAccountBalance from 'core/components/WalletAccountBalance';
import Faucet from 'core/components/Faucet';
import { useNetworks } from 'core/hooks/useNetworks';
import { walletBgColor, walletTextColor } from 'core/colors';
import { useNodeStatus } from 'core/queries/network';
import TransferFlow from 'core/components/TransferFlow';
import { useLocation } from 'react-router-dom';
import WalletAssets from 'core/components/WalletAssets';
import WalletRecentTransactions from 'core/components/WalletRecentTransactions';

function Wallet() {
  const { activeNetwork, faucetClient } = useNetworks();
  const { pathname } = useLocation();

  const { isNodeAvailable } = useNodeStatus(activeNetwork.nodeUrl, {
    refetchInterval: 5000,
  });

  const bgColor = useMemo(() => walletBgColor(pathname), [pathname]);
  const textColor = useMemo(() => walletTextColor(pathname), [pathname]);

  return (
    <WalletLayout title="Home">
      <VStack pb={4} spacing={4} alignItems="stretch">
        <VStack
          py={4}
          px={4}
          color={textColor}
          bgColor={bgColor}
          alignItems="stretch"
        >
          <WalletAccountBalance />
          <SimpleGrid columns={2} spacing={2} pt={4}>
            { faucetClient && <Faucet /> }
            <TransferFlow />
          </SimpleGrid>
        </VStack>
        {
          isNodeAvailable === false ? (
            <Alert status="error">
              <AlertIcon />
              <AlertDescription fontSize="md">
                <Text fontSize="md" fontWeight={700}>Not connected</Text>
                please check your connection
              </AlertDescription>
            </Alert>
          ) : null
        }
        <WalletAssets />
        <WalletRecentTransactions />
      </VStack>
    </WalletLayout>
  );
}

export default Wallet;
