// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from 'react';
import { zxcvbnOptions } from '@zxcvbn-ts/core';
import {
  Button, useColorMode, VStack, Box,
} from '@chakra-ui/react';
import { customColors } from 'core/colors';
import { passwordOptions } from './CreatePasswordBody';
import SecretRecoveryPhraseBody from './SecretRecoveryPhraseBody';
import Copyable from './Copyable';

zxcvbnOptions.setOptions(passwordOptions);

interface Props {
  isLoading: boolean;
  mnemonic: string;
}

const buttonBorderColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

export const buttonBgColor = {
  dark: 'gray.800',
  light: 'white',
};

export default function CreateAccountBody(
  { isLoading, mnemonic }: Props,
) {
  const { colorMode } = useColorMode();
  const [copied, setCopied] = useState<boolean>(false);
  return (
    <Box width="100%">
      <Box display="flex" width="100%" height="100%" px={4}>
        <SecretRecoveryPhraseBody inputHeight={42} />
      </Box>
      <VStack width="100%" spacing={2} pb={4} borderTop="1px" pt={4} px={4} borderColor={buttonBorderColor[colorMode]}>
        <Copyable value={mnemonic} width="100%" copiedPrompt="">
          <Button
            width="100%"
            type="submit"
            border="1px"
            bgColor={buttonBgColor[colorMode]}
            borderColor={customColors.navy[300]}
            isLoading={isLoading}
            px={8}
            onClick={() => setCopied(true)}
          >
            {copied ? 'Copied' : 'Copy'}
          </Button>
        </Copyable>
        <Button
          width="100%"
          colorScheme="teal"
          type="submit"
          isLoading={isLoading}
          px={8}
        >
          Create
        </Button>
      </VStack>
    </Box>
  );
}
