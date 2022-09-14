// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Button,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import { useFormContext } from 'react-hook-form';
import { type MnemonicFormValues } from 'core/layouts/AddAccountLayout';
import MnemonicInput from 'core/components/MnemonicInput';
import { buttonBorderColor } from 'core/colors';

interface ImportAccountMnemonicBodyProps {
  hasSubmit?: boolean;
  px?: number;
}

export default function ImportAccountMnemonicBody({
  hasSubmit,
  px = 4,
}: ImportAccountMnemonicBodyProps) {
  const {
    register,
    setValue,
  } = useFormContext<MnemonicFormValues>();
  const { colorMode } = useColorMode();

  return (
    <VStack spacing={4} px={px} pt={4} height="100%">
      <VStack pt={2} width="100%" spacing={2} flex="1">
        <MnemonicInput register={register} setValue={setValue} />
      </VStack>
      {
        hasSubmit ? (
          <Box py={2} width="100%" borderTop="1px" pt={2} borderColor={buttonBorderColor[colorMode]}>
            <Button colorScheme="teal" width="100%" type="submit">
              Submit
            </Button>
          </Box>
        ) : null
      }
    </VStack>
  );
}
