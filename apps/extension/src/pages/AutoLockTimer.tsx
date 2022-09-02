// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import {
  Box,
  VStack,
  Circle,
  Text,
  Button,
  useColorMode,
  Input,
  InputRightElement,
  InputGroup,
} from '@chakra-ui/react';
import { useAppState } from 'core/hooks/useAppState';
import WalletLayout from 'core/layouts/WalletLayout';
import useAutoLock from 'core/hooks/useAutoLock';
import { iconBgColor, iconColor } from 'core/colors';
import { FaClock } from '@react-icons/all-files/fa/FaClock';
import { Routes } from 'core/routes';

const inputAutolockTimerBgColor = {
  dark: 'gray.800',
  light: 'gray.200',
};

function AutoLockTimer() {
  const { colorMode } = useColorMode();
  const { updateAutoLock } = useAutoLock();
  const navigate = useNavigate();
  const {
    autolockTimer,
  } = useAppState();
  const {
    getValues, register, watch,
  } = useForm({
    defaultValues: {
      timer: (autolockTimer && String(autolockTimer)) || '',
    },
  });

  const timer: string = watch('timer');

  const handleClickSave = () => {
    updateAutoLock(Number(getValues('timer')));
    navigate(Routes.wallet.path);
  };

  return (
    <WalletLayout title="Auto-Lock Timer" showBackButton showAccountCircle={false}>
      <VStack width="100%" height="100%" display="flex" paddingTop={8} px={6}>
        <VStack width="100%" gap={4} flex={1}>
          <Box px={4} width="100%" alignItems="center" display="flex" justifyContent="center">
            <Circle size={16} bgColor={iconBgColor[colorMode]} color={iconColor[colorMode]}>
              <FaClock size={36} />
            </Circle>
          </Box>
          <Text
            fontSize="md"
            textAlign="center"
          >
            How long should we wait to lock your wallet after no activity?
          </Text>
          <InputGroup>
            <Input
              {...register('timer')}
              type="number"
              bgColor={inputAutolockTimerBgColor[colorMode]}
              py={6}
            />
            <InputRightElement width="4.5rem" marginTop={1}>
              <Text color={iconColor[colorMode]}>minutes</Text>
            </InputRightElement>
          </InputGroup>
        </VStack>
        <Button width="full" colorScheme="teal" onClick={handleClickSave} disabled={timer.length === 0}>Save</Button>
      </VStack>
    </WalletLayout>
  );
}

export default AutoLockTimer;
