// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button, Textarea, VStack,
} from '@chakra-ui/react';
import { type PrivateKeyFormValues } from 'core/layouts/AddAccountLayout';
import React from 'react';
import { useFormContext } from 'react-hook-form';

export default function ImportAccountPrivateKeyBody() {
  const {
    register,
  } = useFormContext<PrivateKeyFormValues>();

  return (
    <VStack spacing={4} px={4} pt={4}>
      <Textarea
        variant="filled"
        {...register('privateKey')}
        minLength={1}
        placeholder="Enter your private key here..."
      />
      <Button colorScheme="teal" width="100%" type="submit">
        Import
      </Button>
    </VStack>
  );
}
