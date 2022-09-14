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
      <VStack width="100%" pb={4} spacing={4}>
        <Flex
          py={4}
          px={4}
          width="100%"
          flexDir="column"
          bgColor={bgColor}
        >
          <HStack color={textColor} spacing={0} alignItems="flex-end">
            <WalletAccountBalance />
          </HStack>
          <Flex width="100%" flexDir="column">
            <SimpleGrid columns={2} spacing={2} pt={4}>
              { faucetClient && <Faucet /> }
              <TransferFlow />
            </SimpleGrid>
          </Flex>
        </Flex>
        <WalletAssets />
        {
          isNodeAvailable === false ? (
            <Alert status="error" borderRadius=".5rem">
              <AlertIcon />
              <AlertDescription fontSize="md">
                <Text fontSize="md" fontWeight={700}>Not connected</Text>
                please check your connection
              </AlertDescription>
            </Alert>
          ) : null
        }
      </VStack>
    </WalletLayout>
  );
}

export default Wallet;
