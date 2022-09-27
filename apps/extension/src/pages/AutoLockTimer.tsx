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
import {
  iconBgColor, iconColor, buttonBorderColor, customColors,
} from 'core/colors';
import { FaClock } from '@react-icons/all-files/fa/FaClock';
import { Routes } from 'core/routes';

const inputAutolockTimerBgColor = {
  dark: 'gray.800',
  light: customColors.navy[100],
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
      <VStack width="100%" height="100%" display="flex" paddingTop={8}>
        <VStack width="100%" gap={4} flex={1} px={4}>
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
        <Box width="100%" borderTop="1px" pt={4} px={4} borderColor={buttonBorderColor[colorMode]}>
          <Button width="100%" colorScheme="salmon" height="48px" onClick={handleClickSave} disabled={timer.length === 0}>Save</Button>
        </Box>
      </VStack>
    </WalletLayout>
  );
}

export default AutoLockTimer;
