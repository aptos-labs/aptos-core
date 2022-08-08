// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
  Input,
  InputGroup,
  InputLeftAddon,
  SimpleGrid,
  VStack,
} from '@chakra-ui/react';
import { AptosAccount } from 'aptos';
import useWalletState from 'core/hooks/useWalletState';
import { getAccountResources } from 'core/queries/account';
import Routes from 'core/routes';
import { generateMnemonicObject } from 'core/utils/account';
import Analytics from 'core/utils/analytics/analytics';
import { loginEvents } from 'core/utils/analytics/events';
import React, { useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';

interface FormValues {
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

export default function ImportAccountMnemonicBody() {
  const { addAccount, nodeUrl } = useWalletState();
  const navigate = useNavigate();
  const {
    handleSubmit, register, setError, watch,
  } = useForm<FormValues>();

  const [isLoading, setIsLoading] = useState<boolean>(false);

  const mnemonicAll = watch();

  const onSubmit: SubmitHandler<FormValues> = async (data, event) => {
    setIsLoading(true);
    event?.preventDefault();
    let mnemonicString = '';
    Object.values(mnemonicAll).forEach((value) => {
      mnemonicString = `${mnemonicString + value} `;
    });
    mnemonicString = mnemonicString.trim();

    try {
      setIsLoading(true);
      const mnemonicObject = await generateMnemonicObject(mnemonicString);
      const aptosAccount = new AptosAccount(mnemonicObject.seed);
      const response = await getAccountResources({
        address: aptosAccount.address().hex(),
        nodeUrl,
      });
      setIsLoading(false);
      const analyticsParams = {
        address: aptosAccount.address().hex(),
        network: nodeUrl,
      };
      if (!response) {
        setError('mnemonic-a', { message: 'Invalid mnemonic, account not found', type: 'custom' });
        Analytics.event({
          eventType: loginEvents.ERROR_LOGIN_WITH_PRIVATE_KEY,
          params: analyticsParams,
        });
        return;
      }
      Analytics.event({
        eventType: loginEvents.LOGIN_WITH_PRIVATE_KEY,
        params: analyticsParams,
      });
      await addAccount({ account: aptosAccount, isImport: true, mnemonic: mnemonicObject });
      setIsLoading(false);
      navigate(Routes.wallet.routePath);
    } catch (err) {
      setIsLoading(false);
      Analytics.event({
        eventType: loginEvents.ERROR_LOGIN_WITH_PRIVATE_KEY,
        params: {
          network: nodeUrl,
        },
      });
      setError('mnemonic-a', { message: 'Invalid mnemonic, account not found', type: 'custom' });
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <VStack spacing={4} px={4} pt={4}>
        <VStack width="100%">
          <SimpleGrid columns={2} gap={4}>
            <VStack>
              <InputGroup size="sm">
                <InputLeftAddon>1</InputLeftAddon>
                <Input {...register('mnemonic-a')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>2</InputLeftAddon>
                <Input {...register('mnemonic-b')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>3</InputLeftAddon>
                <Input {...register('mnemonic-c')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>4</InputLeftAddon>
                <Input {...register('mnemonic-d')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>5</InputLeftAddon>
                <Input {...register('mnemonic-e')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>6</InputLeftAddon>
                <Input {...register('mnemonic-f')} isRequired variant="outline" />
              </InputGroup>
            </VStack>
            <VStack>
              <InputGroup size="sm">
                <InputLeftAddon>7</InputLeftAddon>
                <Input {...register('mnemonic-g')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>8</InputLeftAddon>
                <Input {...register('mnemonic-h')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>9</InputLeftAddon>
                <Input {...register('mnemonic-i')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>10</InputLeftAddon>
                <Input {...register('mnemonic-j')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>11</InputLeftAddon>
                <Input {...register('mnemonic-k')} isRequired variant="outline" />
              </InputGroup>
              <InputGroup size="sm">
                <InputLeftAddon>12</InputLeftAddon>
                <Input {...register('mnemonic-l')} isRequired variant="outline" />
              </InputGroup>
            </VStack>
          </SimpleGrid>
        </VStack>
        <Button isLoading={isLoading} colorScheme="teal" width="100%" type="submit">
          Submit
        </Button>
      </VStack>
    </form>
  );
}
