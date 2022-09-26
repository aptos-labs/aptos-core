// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  VStack,
  useColorMode,
  Heading,
  Flex,
  Text,
  Button,
} from '@chakra-ui/react';
import Routes from 'core/routes';
import React from 'react';
import { AptosBlackLogo, AptosWhiteLogo } from './AptosLogo';
import ChakraLink from './ChakraLink';

export default function NewExtensionBody() {
  const { colorMode } = useColorMode();

  return (
    <VStack height="100%">
      <Flex w="100%" flexDir="column" height="100%">
        <Flex w="100%" flexDir="column" flex={1}>
          <Flex w="100%" flexDir="column" margin="auto">
            <Box width="86px" pb={5}>
              {
              (colorMode === 'dark')
                ? <AptosWhiteLogo />
                : <AptosBlackLogo />
            }
            </Box>
            <Heading
              size="lg"
              fontWeight={700}
            >
              Welcome to Petra
            </Heading>
            <Text
              pb={10}
              pt={2}
              fontSize="md"
            >
              Guiding your web3 journey
            </Text>
          </Flex>
        </Flex>
        <VStack spacing={6}>
          <ChakraLink to={Routes.createWallet.path} width="100%">
            <Button colorScheme="salmon" color="white" variant="solid" width="100%" height={14}>
              <Text
                fontSize="xl"
              >
                Create New Wallet
              </Text>
            </Button>
          </ChakraLink>
          <ChakraLink to={Routes.createWalletViaImportAccount.path} width="100%">
            <Button variant="solid" width="100%" height={14}>
              <Text
                fontSize="xl"
              >
                Import Wallet
              </Text>
            </Button>
          </ChakraLink>
        </VStack>
      </Flex>
    </VStack>
  );
}
