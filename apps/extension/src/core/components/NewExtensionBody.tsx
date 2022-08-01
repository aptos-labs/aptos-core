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
        <Heading textAlign="center">Wallet</Heading>
        <Text
          textAlign="center"
          pb={8}
          color={secondaryExtensionBodyTextColor[colorMode]}
          fontSize="lg"
        >
          An Aptos crypto wallet
        </Text>
        <VStack spacing={4}>
          <ChakraLink to="/create-wallet" width="100%">
            <Button colorScheme="teal" variant="solid" width="100%">
              Create a new wallet
            </Button>
          </ChakraLink>
          <ChakraLink to="/import" width="100%">
            <Button colorScheme="gray" variant="solid" width="100%">
              I already have a wallet
            </Button>
          </ChakraLink>
        </VStack>
      </Flex>
    </VStack>
  );
}
