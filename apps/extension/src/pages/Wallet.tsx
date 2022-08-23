// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Alert,
  AlertDescription,
  AlertIcon,
  Button,
  Flex,
  HStack,
  useColorMode,
  VStack,
  Text,
  Tooltip,
  IconButton,
  useClipboard,
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
import { useNodeStatus } from 'core/queries/network';
import { BiCopy } from '@react-icons/all-files/bi/BiCopy';

function CopyAddressButton() {
  const { activeAccountAddress } = useGlobalStateContext();
  const { hasCopied, onCopy } = useClipboard(activeAccountAddress ?? '');
  return (
    <Tooltip label={hasCopied ? 'Copied!' : 'Copy Address'} closeDelay={300}>
      <IconButton
        fontSize="24px"
        icon={<BiCopy />}
        aria-label="Copy Address"
        bg="clear"
        _focus={{ boxShadow: 'none' }}
        _active={{
          bg: 'none',
          transform: 'scale(0.90)',
        }}
        onClick={onCopy}
      />
    </Tooltip>
  );
}

function Wallet() {
  const { colorMode } = useColorMode();
  const { activeNetwork, faucetClient } = useGlobalStateContext();
  const { isNodeAvailable } = useNodeStatus(activeNetwork?.nodeUrl, {
    refetchInterval: 5000,
  });

  return (
    <AuthLayout routePath={PageRoutes.wallet.path}>
      <WalletLayout accessoryButton={<CopyAddressButton />} title="Home">
        <VStack width="100%" p={4}>
          <Flex
            py={4}
            width="100%"
            flexDir="column"
            borderRadius=".5rem"
            bgColor={secondaryWalletHomeCardBgColor[colorMode]}
          >
            <HStack spacing={0} alignItems="flex-end">
              <WalletAccountBalance />
            </HStack>
            <Flex width="100%" flexDir="column" px={4}>
              <HStack spacing={4} pt={4}>
                { faucetClient && <Faucet /> }
                <TransferDrawer />
              </HStack>
            </Flex>
          </Flex>
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
    </AuthLayout>

  );
}

export default Wallet;
