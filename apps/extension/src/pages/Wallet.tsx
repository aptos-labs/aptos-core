// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Alert,
  AlertDescription,
  AlertIcon,
  Button,
  Flex,
  HStack,
  VStack,
  Text,
  Tooltip,
  IconButton,
  useClipboard,
  SimpleGrid,
} from '@chakra-ui/react';
import React, { useMemo } from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import WalletAccountBalance from 'core/components/WalletAccountBalance';
import Faucet from 'core/components/Faucet';
import Routes from 'core/routes';
import { useNetworks } from 'core/hooks/useNetworks';
import { walletBgColor, walletTextColor } from 'core/colors';
import { ChevronRightIcon } from '@chakra-ui/icons';
import ChakraLink from 'core/components/ChakraLink';
import { useNodeStatus } from 'core/queries/network';
import { BiCopy } from '@react-icons/all-files/bi/BiCopy';
import { useActiveAccount } from 'core/hooks/useAccounts';
import WalletAccountStake from 'core/components/WalletAccountStake';
import TransferFlow from 'core/components/TransferFlow';
import { useLocation } from 'react-router-dom';

function CopyAddressButton() {
  const { activeAccountAddress } = useActiveAccount();
  const { hasCopied, onCopy } = useClipboard(activeAccountAddress ?? '');
  return (
    <Tooltip label={hasCopied ? 'Copied!' : 'Copy Address'} closeDelay={300}>
      <IconButton
        fontSize="20px"
        icon={<BiCopy />}
        aria-label="Copy Address"
        bg="clear"
        _focus={{ boxShadow: 'none' }}
        _active={{
          bg: 'none',
          transform: 'scale(0.90)',
        }}
        onClick={onCopy}
        variant="none"
      />
    </Tooltip>
  );
}

function Wallet() {
  const { activeNetwork, faucetClient } = useNetworks();
  const { pathname } = useLocation();

  const { isNodeAvailable } = useNodeStatus(activeNetwork.nodeUrl, {
    refetchInterval: 5000,
  });

  const bgColor = useMemo(() => walletBgColor(pathname), [pathname]);
  const textColor = useMemo(() => walletTextColor(pathname), [pathname]);

  return (
    <WalletLayout accessoryButton={<CopyAddressButton />} title="Home">
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
        <ChakraLink px={4} width="100%" to={Routes.stake.path}>
          <Button
            py={10}
            width="100%"
            rightIcon={<ChevronRightIcon />}
            justifyContent="space-between"
          >
            <WalletAccountStake />
          </Button>
        </ChakraLink>
        <ChakraLink px={4} width="100%" to={Routes.activity.path}>
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
  );
}

export default Wallet;
