// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { useFormContext } from 'react-hook-form';
import { zxcvbnOptions } from '@zxcvbn-ts/core';
import {
  Box, Button, VStack,
} from '@chakra-ui/react';
import { type CreateAccountFormValues } from 'core/layouts/AddAccountLayout';
import { passwordOptions } from './CreatePasswordBody';
import SecretRecoveryPhraseBody from './SecretRecoveryPhraseBody';

zxcvbnOptions.setOptions(passwordOptions);

export default function CreateAccountBody() {
  const { watch } = useFormContext<CreateAccountFormValues>();
  const secretRecoveryPhrase = watch('secretRecoveryPhrase');

  return (
    <Box width="100%" height="100%" px={4}>
      <VStack spacing={4}>
        <SecretRecoveryPhraseBody />
        <Button
          colorScheme="teal"
          type="submit"
          isDisabled={!secretRecoveryPhrase}
          px={8}
        >
          Submit
        </Button>
      </VStack>
    </Box>
  );
}
