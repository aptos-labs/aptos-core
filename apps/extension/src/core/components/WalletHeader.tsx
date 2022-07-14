// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Grid,
  HStack,
  Text,
  useColorMode,
} from '@chakra-ui/react';
import React from 'react';
import useWalletState from 'core/hooks/useWalletState';
import { ChevronLeftIcon } from '@chakra-ui/icons';
import Copyable from 'core/components/Copyable';
import { collapseHexString } from 'core/utils/hex';
import ChakraLink from './ChakraLink';

const secondaryHeaderBgColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

export const secondaryAddressFontColor = {
  dark: 'gray.400',
  light: 'gray.500',
};

interface WalletHeaderProps {
  backPage?: string;
}

export default function WalletHeader({
  backPage,
}: WalletHeaderProps) {
  const { aptosAccount } = useWalletState();
  const { colorMode } = useColorMode();

  return (
    <Grid
      maxW="100%"
      width="100%"
      py={2}
      height="40px"
      templateColumns="40px 1fr 40px"
      bgColor={secondaryHeaderBgColor[colorMode]}
    >
      {(backPage) ? (
        <Center>
          <ChakraLink to={backPage}>
            <ChevronLeftIcon fontSize="xl" aria-label={backPage} />
          </ChakraLink>
        </Center>
      ) : <Box />}
      <Center>
        <HStack px={2}>
          <Text
            fontSize="xs"
            color={secondaryAddressFontColor[colorMode]}
          >
            Address
          </Text>
          <Copyable value={aptosAccount!.address()}>
            <Text whiteSpace="nowrap" as="span">
              <Text
                fontSize="xs"
                as="span"
                whiteSpace="nowrap"
                overflow="hidden"
                display="block"
                noOfLines={1}
                maxW={['100px', '120px']}
              >
                { collapseHexString(aptosAccount!.address(), 12) }
              </Text>
            </Text>
          </Copyable>
        </HStack>
      </Center>
      <Box />
    </Grid>
  );
}
