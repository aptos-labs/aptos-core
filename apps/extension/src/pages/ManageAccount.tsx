// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import {
  VStack, Button, Text, Input, useColorMode, Box, HStack, Flex, Icon, ButtonGroup, Textarea,
} from '@chakra-ui/react';
import { useForm } from 'react-hook-form';
import { RiRotateLockFill } from '@react-icons/all-files/ri/RiRotateLockFill';
import { RiFileCopyLine } from '@react-icons/all-files/ri/RiFileCopyLine';
import { useActiveAccount } from 'core/hooks/useAccounts';
import Copyable from 'core/components/Copyable';
import {
  secondaryTextColor, buttonBorderColor, rotationKeyButtonBgColor, customColors,
} from 'core/colors';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';

export default function ManageAccount() {
  const { colorMode } = useColorMode();
  const navigate = useNavigate();
  const {
    watch,
  } = useForm({
    defaultValues: {
      isLoading: false,
      showPrivateKey: false,
      showRecoveryPhrase: false,
    },
  });
  const { activeAccount } = useActiveAccount();

  const isLoading = watch('isLoading');

  const handleRotateKey = async () => {
    navigate(Routes.rotate_key_onboarding.path);
  };

  return (
    <WalletLayout title="Manage Account" showBackButton showAccountCircle={false}>
      <VStack py={4} height="100%" spacing={4} px={4}>
        {activeAccount.mnemonic && (
        <VStack width="100%" borderBottom="1px" borderColor={buttonBorderColor[colorMode]} paddingBottom={4}>
          <HStack alignContent="center" width="100%">
            <Text
              fontSize="16"
              fontWeight={700}
              flex={1}
            >
              Show secret recovery phrase
            </Text>
            <Button
              size="sm"
              bgColor={rotationKeyButtonBgColor[colorMode]}
              border="1px"
              borderColor={customColors.navy[200]}
              onClick={() => navigate(Routes.manage_account_show_recovery_phrase.path)}
            >
              Show
            </Button>
          </HStack>
        </VStack>
        )}
        <Box width="100%" borderBottom="1px" borderColor={buttonBorderColor[colorMode]} paddingBottom={8}>
          <HStack alignItems="flex-start" width="100%">
            <Text
              fontSize={16}
              fontWeight={700}
              flex={1}
              minWidth={24}
            >
              Private Key
            </Text>
            <Input
              marginTop={4}
              color={secondaryTextColor[colorMode]}
              bgColor={rotationKeyButtonBgColor[colorMode]}
              size="xs"
              type="password"
              readOnly
              variant="filled"
              fontSize={16}
              value={activeAccount.privateKey}
            />
          </HStack>
          <Flex marginTop={4} justifyContent="flex-end">
            <ButtonGroup>
              <Button
                size="sm"
                bgColor={rotationKeyButtonBgColor[colorMode]}
                onClick={() => navigate(Routes.manage_account_show_private_key.path)}
              >
                Show
              </Button>
              <Button
                size="sm"
                onClick={handleRotateKey}
                isLoading={isLoading}
                bgColor={rotationKeyButtonBgColor[colorMode]}
                border="1px"
                borderColor={customColors.navy[200]}
                leftIcon={<RiRotateLockFill />}
              >
                Rotate key
              </Button>
            </ButtonGroup>
          </Flex>
        </Box>
        <Box width="100%">
          <Flex>
            <Text
              fontSize="md"
              fontWeight={700}
              flex={1}
            >
              Public Key
            </Text>
            <Copyable
              prompt="Copy public key"
              value={activeAccount.publicKey}
            >
              <HStack alignItems="center">
                <Text
                  fontSize={13}
                  fontWeight={500}
                >
                  Copy
                </Text>
                <Icon as={RiFileCopyLine} my="auto" w={4} h={4} margin="auto" />
              </HStack>
            </Copyable>
          </Flex>
          <Textarea
            marginTop={4}
            color={secondaryTextColor[colorMode]}
            height={20}
            readOnly
            variant="filled"
            fontSize={16}
            value={activeAccount.publicKey}
          />
        </Box>
      </VStack>
    </WalletLayout>
  );
}
