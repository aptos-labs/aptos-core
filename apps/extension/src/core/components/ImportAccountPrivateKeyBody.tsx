// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button, Textarea, VStack,
} from '@chakra-ui/react';
import { type PrivateKeyFormValues } from 'core/layouts/AddAccountLayout';
import React from 'react';
import { useFormContext } from 'react-hook-form';

interface ImportAccountPrivateKeyBodyProps {
  hasSubmit?: boolean;
  px?: number;
}

export default function ImportAccountPrivateKeyBody({
  hasSubmit = false,
  px = 4,
}: ImportAccountPrivateKeyBodyProps) {
  const {
    register,
  } = useFormContext<PrivateKeyFormValues>();

  return (
    <VStack spacing={4} px={px} pt={4}>
      <Textarea
        variant="filled"
        {...register('privateKey')}
        minLength={1}
        placeholder="Enter your private key here..."
      />
      {
        hasSubmit ? (
          <Button colorScheme="teal" width="100%" type="submit">
            Import
          </Button>
        ) : null
      }
    </VStack>
  );
}
