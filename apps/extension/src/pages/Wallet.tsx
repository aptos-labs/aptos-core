// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Button,
  Flex,
  HStack,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import WalletAccountBalance from 'core/components/WalletAccountBalance';
import TransferDrawer from 'core/components/TransferDrawer';
import Faucet from 'core/components/Faucet';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import { secondaryWalletHomeCardBgColor } from 'core/colors';
import { ChevronRightIcon } from '@chakra-ui/icons';
import ChakraLink from 'core/components/ChakraLink';

function Wallet() {
  const { colorMode } = useColorMode();
  const { faucetClient } = useGlobalStateContext();

  return (
    <AuthLayout routePath={PageRoutes.wallet.path}>
      <WalletLayout>
        <VStack width="100%" paddingTop={4}>
          <Flex px={4} width="100%">
            <Flex
              py={4}
              width="100%"
              flexDir="column"
              borderRadius=".5rem"
              bgColor={secondaryWalletHomeCardBgColor[colorMode]}
            >
              <HStack spacing={0} alignItems="flex-end">
                <WalletAccountBalance />
                <Box pb="2px">
                  <Text fontSize="xl" fontWeight={600}>
                    APT
                  </Text>
                </Box>
              </HStack>
              <Flex width="100%" flexDir="column" px={4}>
                <HStack spacing={4} pt={4}>
                  { faucetClient && <Faucet /> }
                  <TransferDrawer />
                </HStack>
              </Flex>
            </Flex>
          </Flex>
          <Flex width="100%" px={4}>
            <ChakraLink width="100%" to={PageRoutes.activity.path}>
              <Button
                py={6}
                width="100%"
                rightIcon={<ChevronRightIcon />}
                justifyContent="space-between"
              >
                View your activity
              </Button>
            </ChakraLink>
          </Flex>
        </VStack>
      </WalletLayout>
    </AuthLayout>

  );
}

export default Wallet;
