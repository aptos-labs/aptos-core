// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect } from 'react';
import {
  Heading,
  Center,
  Box,
  Text,
  VStack,
} from '@chakra-ui/react';

import { AptosBlackLogo } from 'core/components/AptosLogo';
import { usePermissionRequestContext } from '../hooks';

export function NoAccounts() {
  const { reject } = usePermissionRequestContext();

  useEffect(() => {
    reject();
  }, [reject]);

  return (
    <VStack
      h="100%"
      w="100%"
      alignItems="center"
      justifyContent="center"
      padding={8}
    >
      <Center>
        <Box width="75px">
          <AptosBlackLogo />
        </Box>
      </Center>
      <Heading textAlign="center">Petra</Heading>
      <Text
        textAlign="center"
        pb={8}
        fontSize="lg"
      >
        Please open the extension and create an account.
      </Text>
    </VStack>
  );
}

export default NoAccounts;
