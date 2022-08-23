// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  useColorMode,
  Box,
  VStack,
  Center,
  Heading,
  Flex,
  Text,
  Button,
} from '@chakra-ui/react';
import { secondaryExtensionBodyTextColor } from 'core/colors';
import Routes from 'core/routes';
import React from 'react';
import { AptosBlackLogo, AptosWhiteLogo } from './AptosLogo';
import ChakraLink from './ChakraLink';

export default function NewExtensionBody() {
  const { colorMode } = useColorMode();

  return (
    <VStack height="100%" pt={32}>
      <Flex w="100%" flexDir="column">
        <Center>
          <Box width="75px" pb={4}>
            {
              (colorMode === 'dark')
                ? <AptosWhiteLogo />
                : <AptosBlackLogo />
            }
          </Box>
        </Center>
        <Heading textAlign="center">Petra</Heading>
        <Text
          textAlign="center"
          pb={8}
          color={secondaryExtensionBodyTextColor[colorMode]}
          fontSize="lg"
        >
          An Aptos crypto wallet
        </Text>
        <VStack spacing={4}>
          <ChakraLink to={Routes.createWallet.path} width="100%">
            <Button colorScheme="teal" variant="solid" width="100%">
              Get started
            </Button>
          </ChakraLink>
          <ChakraLink to={Routes.createWalletViaImportAccount.path} width="100%">
            <Button variant="solid" width="100%">
              Import account
            </Button>
          </ChakraLink>
        </VStack>
      </Flex>
    </VStack>
  );
}
