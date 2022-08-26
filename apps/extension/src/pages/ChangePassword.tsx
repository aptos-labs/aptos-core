// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Box,
  VStack,
  Circle,
  Button,
  useColorMode,
  Text,
  Input,
  InputRightElement,
  InputGroup,
} from '@chakra-ui/react';
import { useInitializedAccounts } from 'core/hooks/useAccounts';
import WalletLayout from 'core/layouts/WalletLayout';
import { FaLock } from '@react-icons/all-files/fa/FaLock';
import { AiOutlineEye } from '@react-icons/all-files/ai/AiOutlineEye';
import { AiOutlineEyeInvisible } from '@react-icons/all-files/ai/AiOutlineEyeInvisible';
import {
  changePasswordNewPasswordNotMatchErrorToast,
  changePasswordSuccessfullyUpdatedToast,
  changePasswordIncorrectCurrentPasswordErrorToast,
} from 'core/components/Toast';
import { Routes } from 'core/routes';
import { inputChangePasswordBgColor, lockIconBgColor, lockIconColor } from 'core/colors';

function ChangePassword() {
  const [currentPassword, setCurrentPassword] = useState<string>('');
  const [newPassword, setNewPassword] = useState<string>('');
  const [confirmNewPassword, setConfirmNewPassword] = useState<string>('');
  const [show, setShow] = useState<boolean>(false);
  const { changePassword } = useInitializedAccounts();
  const navigate = useNavigate();
  const { colorMode } = useColorMode();

  const handleClickShow = () => setShow(!show);

  const handleClickSave = async () => {
    if (newPassword !== confirmNewPassword) {
      changePasswordNewPasswordNotMatchErrorToast();
      return;
    }

    try {
      await changePassword(currentPassword, newPassword);
      changePasswordSuccessfullyUpdatedToast();
      navigate(Routes.wallet.path);
    } catch (e) {
      changePasswordIncorrectCurrentPasswordErrorToast();
    }
  };

  const shouldDisableSaveButton = currentPassword.length === 0
  || newPassword.length === 0
  || confirmNewPassword.length === 0;

  return (
    <WalletLayout title="Change password" showBackButton showAccountCircle={false}>
      <VStack width="100%" height="100%" display="flex" paddingTop={8} px={6}>
        <VStack width="100%" gap={4} flex={1}>
          <Box px={4} pb={0} width="100%" alignItems="center" display="flex" justifyContent="center">
            <Circle size={16} bgColor={lockIconBgColor[colorMode]} color={lockIconColor[colorMode]}>
              <FaLock size={36} />
            </Circle>
          </Box>
          <Text
            fontSize="md"
            textAlign="center"
            as="div"
          >
            You&apos;ll use this to unlock your wallet
          </Text>
          <InputGroup>
            <Input
              placeholder="Current password"
              type={show ? 'text' : 'password'}
              value={currentPassword}
              onChange={(e) => setCurrentPassword(e.target.value)}
              bgColor={inputChangePasswordBgColor[colorMode]}
              paddingTop={6}
              paddingBottom={6}
            />
            <InputRightElement width="4.5rem" marginTop={1}>
              {show
                ? <AiOutlineEyeInvisible size={32} onClick={handleClickShow} />
                : <AiOutlineEye size={32} onClick={handleClickShow} />}
            </InputRightElement>
          </InputGroup>
          <Input
            placeholder="New password"
            type={show ? 'text' : 'password'}
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            bgColor={inputChangePasswordBgColor[colorMode]}
            paddingTop={6}
            paddingBottom={6}
          />
          <Input
            placeholder="Confirm new password"
            type={show ? 'text' : 'password'}
            value={confirmNewPassword}
            onChange={(e) => setConfirmNewPassword(e.target.value)}
            bgColor={inputChangePasswordBgColor[colorMode]}
            paddingTop={6}
            paddingBottom={6}
          />
        </VStack>
        <Button width="full" colorScheme="teal" height={14} onClick={handleClickSave} disabled={shouldDisableSaveButton}>Save</Button>
      </VStack>
    </WalletLayout>
  );
}

export default ChangePassword;
