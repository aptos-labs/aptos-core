// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  FormControl,
  Input,
  VStack,
  Button,
  FormErrorMessage,
  Flex,
  Text,
  useColorMode,
} from '@chakra-ui/react';
import React from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';
import { buttonBorderColor, secondaryButtonBgColor, customColors } from 'core/colors';
import { useActiveAccount, useUnlockedAccounts } from 'core/hooks/useAccounts';

interface AccountEditFormData {
  name: string,
}

export default function RenameAccountBody() {
  const { renameAccount } = useUnlockedAccounts();
  const { activeAccount } = useActiveAccount();
  const { address, name } = activeAccount!;
  const { colorMode } = useColorMode();
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
    <form onSubmit={handleSubmit(onSubmit)} style={{ height: '100%' }}>
      <Flex flexDirection="column" height="100%">
        <FormControl isInvalid={errors.name !== undefined} display="flex" gap={2} flexDirection="column" flex={1} px={4} pb={4}>
          <Text fontSize={18}>Wallet Name</Text>
          <Input
            height="48px"
            placeholder="Enter wallet name"
            required
            {...register('name', {
              maxLength: { message: 'Too long', value: 20 },

            })}
          />
          <FormErrorMessage>{ errors.name?.message }</FormErrorMessage>
        </FormControl>
        <VStack
          width="100%"
          px={4}
          borderTop="1px"
          paddingTop={4}
          spacing={2}
          borderColor={buttonBorderColor[colorMode]}
        >
          <Button
            width="100%"
            height="48px"
            isDisabled={!isValid}
            type="submit"
            colorScheme="salmon"
          >
            Save
          </Button>
          <Button
            width="100%"
            height="48px"
            type="submit"
            bgColor={secondaryButtonBgColor[colorMode]}
            border="1px"
            borderColor={customColors.navy[400]}
          >
            Cancel
          </Button>
        </VStack>
      </Flex>
    </form>
  );
}
