// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box, Grid, useColorMode,
} from '@chakra-ui/react';
import { secondaryBgColor } from 'core/colors';
import ImportAccountHeader from 'core/components/ImportAccountHeader';
import React from 'react';
import {
  DeepPartial, FormProvider, useForm,
} from 'react-hook-form';

export interface AddAccountLayoutProps<T> {
  backPage?: string;
  children: React.ReactNode;
  defaultValues: DeepPartial<T>,
  headerValue?: string;
  onSubmit: (data: T, event?: React.BaseSyntheticEvent) => void
}

export default function AddAccountLayout<T>({
  backPage,
  children,
  defaultValues,
  headerValue,
  onSubmit,
}: AddAccountLayoutProps<T>) {
  const { colorMode } = useColorMode();
  const methods = useForm<T>({
    defaultValues,
  });

  const { handleSubmit } = methods;

  return (
    <Grid
      height="100%"
      width="100%"
      maxW="100%"
      templateRows="60px 1fr"
      position="relative"
      bgColor={secondaryBgColor[colorMode]}
    >
      <ImportAccountHeader backPage={backPage} headerValue={headerValue} />
      <Box height="100%" width="100%" maxH="100%" overflowY="auto">
        <FormProvider {...methods}>
          <form onSubmit={handleSubmit(onSubmit)} style={{ height: '100%' }}>
            {children}
          </form>
        </FormProvider>
      </Box>
    </Grid>
  );
}

export interface CreateAccountFormValues {
  mnemonic: string[];
  mnemonicString: string;
}

export interface PrivateKeyFormValues {
  privateKey: string;
}

export interface MnemonicFormValues {
  'mnemonic-a': string;
  'mnemonic-b': string;
  'mnemonic-c': string;
  'mnemonic-d': string;
  'mnemonic-e': string;
  'mnemonic-f': string;
  'mnemonic-g': string;
  'mnemonic-h': string;
  'mnemonic-i': string;
  'mnemonic-j': string;
  'mnemonic-k': string;
  'mnemonic-l': string;
}

export const CreateAccountLayout = (
  props: AddAccountLayoutProps<CreateAccountFormValues>,
) => AddAccountLayout<CreateAccountFormValues>(props);

export const ImportAccountMnemonicLayout = (
  props: AddAccountLayoutProps<MnemonicFormValues>,
) => AddAccountLayout<MnemonicFormValues>(props);

export const ImportAccountPrivateKeyLayout = (
  props: AddAccountLayoutProps<PrivateKeyFormValues>,
) => AddAccountLayout<PrivateKeyFormValues>(props);
