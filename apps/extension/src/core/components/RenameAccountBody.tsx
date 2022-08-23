// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  FormControl,
  Input,
  Button,
  FormErrorMessage,
} from '@chakra-ui/react';
import React from 'react';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';

interface AccountEditFormData {
  name: string,
}

export default function RenameAccountBody() {
  const { activeAccount, renameAccount } = useGlobalStateContext();
  const { address, name } = activeAccount!;
  const navigate = useNavigate();

  const {
    formState: { errors, isValid },
    handleSubmit,
    register,
  } = useForm<AccountEditFormData>({
    defaultValues: { name },
    mode: 'onChange',
  });

  const onSubmit: SubmitHandler<AccountEditFormData> = async (data, event) => {
    event?.preventDefault();
    await renameAccount(address, data.name);
    navigate(Routes.wallet.path);
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)}>
      <FormControl isInvalid={errors.name !== undefined}>
        <Input
          placeholder="Insert account name"
          {...register('name', {
            maxLength: { message: 'Too long', value: 20 },
            required: 'Please insert a valid name',
          })}
        />
        <FormErrorMessage>{ errors.name?.message }</FormErrorMessage>
      </FormControl>
      <Button
        isDisabled={!isValid}
        type="submit"
        colorScheme="teal"
        mt={3}
      >
        Confirm
      </Button>
    </form>
  );
}
