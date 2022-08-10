// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Button,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { useWalletState } from 'core/hooks/useWalletState';
import React from 'react';
import Copyable from 'core/components/Copyable';
import { BiCopy } from '@react-icons/all-files/bi/BiCopy';
import { secondaryBorderColor } from 'core/colors';

export default function RecoveryPhraseBox() {
  const { accountMnemonic } = useWalletState();
  const { colorMode } = useColorMode();
  return (
    <VStack align="flex-start">
      <Box width="100%" borderRadius=".5rem" borderWidth="1px" borderColor={secondaryBorderColor[colorMode]} p={4} rounded="md" bg="white">
        <Text>
          {accountMnemonic?.mnemonic}
        </Text>
      </Box>
      <Copyable
        prompt="Copy phrase"
        value={accountMnemonic?.mnemonic}
      >
        <Button
          justifyContent="flex-start"
          leftIcon={<BiCopy />}
          fontSize="sm"
          bg="clear"
          _hover={{ bg: 'none' }}
          _focus={{ boxShadow: 'none' }}
          _active={{
            bg: 'none',
            transform: 'scale(0.90)',
          }}
        >
          Copy phrase
        </Button>
      </Copyable>
    </VStack>

  );
}
