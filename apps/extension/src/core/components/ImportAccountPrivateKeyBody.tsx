// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button, Textarea, VStack,
} from '@chakra-ui/react';
import { AptosAccount } from 'aptos';
import useWalletState from 'core/hooks/useWalletState';
import { getAccountResources } from 'core/queries/account';
import Routes from 'core/routes';
import Analytics from 'core/utils/analytics/analytics';
import { loginEvents } from 'core/utils/analytics/events';
import React from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';

interface FormValues {
  privateKey: string;
}

export default function ImportAccountPrivateKeyBody() {
  const { addAccount, nodeUrl } = useWalletState();
  const navigate = useNavigate();
  const {
    handleSubmit, register, setError, watch,
  } = useForm<FormValues>();
  const privateKey = watch('privateKey');

  const onSubmit: SubmitHandler<FormValues> = async (data, event) => {
    event?.preventDefault();
    try {
      const nonHexKey = (privateKey.startsWith('0x')) ? privateKey.substring(2) : privateKey;
      const encodedKey = Uint8Array.from(Buffer.from(nonHexKey, 'hex'));
      const account = new AptosAccount(encodedKey, undefined);
      const response = await getAccountResources({
        address: account.address().hex(),
        nodeUrl,
      });
      const analyticsParams = {
        address: account.address().hex(),
        network: nodeUrl,
      };
      if (!response) {
        setError('privateKey', { message: 'Account not found', type: 'custom' });
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
      await addAccount({ account, isImport: true });
      navigate(Routes.wallet.routePath);
    } catch (err) {
      Analytics.event({
        eventType: loginEvents.ERROR_LOGIN_WITH_PRIVATE_KEY,
        params: {
          network: nodeUrl,
        },
      });
      setError('privateKey', { message: 'Invalid private key', type: 'custom' });
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <VStack spacing={4} px={4} pt={4}>
        <Textarea
          variant="filled"
          {...register('privateKey')}
          placeholder="Enter your private key here..."
        />
        <Button colorScheme="teal" width="100%" type="submit">
          Submit
        </Button>
      </VStack>
    </form>
  );
}
