// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Center,
  Grid,
  Text,
  useColorMode,
} from '@chakra-ui/react';
import React from 'react';
import {
  ChevronLeftIcon,
} from '@chakra-ui/icons';
import {
  secondaryBorderColor,
} from 'core/colors';
import ChakraLink from './ChakraLink';

interface WalletHeaderProps {
  backPage?: string;
  headerValue?: string;
}

export default function ImportAccountHeader({
  backPage,
  headerValue = 'Import wallet',
}: WalletHeaderProps) {
  const { colorMode } = useColorMode();

  return (
    <Grid
      maxW="100%"
      width="100%"
      py={4}
      height="64px"
      templateColumns={backPage ? '40px 1fr 40px' : '1fr'}
      borderBottomColor={secondaryBorderColor[colorMode]}
      borderBottomWidth="1px"
    >
      {(backPage) ? (
        <Center>
          <ChakraLink to={backPage}>
            <ChevronLeftIcon fontSize="xl" aria-label={backPage} />
          </ChakraLink>
        </Center>
      ) : <Box />}
      <Center width="100%">
        <Text fontWeight={600}>
          {headerValue}
        </Text>
      </Center>
      <Box />
    </Grid>
  );
}
