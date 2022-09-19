// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import {
  Box,
  VStack,
  Button,
  useColorMode,
} from '@chakra-ui/react';
import { useNavigate, useLocation } from 'react-router-dom';
import { FormProvider, useForm } from 'react-hook-form';
import WalletLayout from 'core/layouts/WalletLayout';
import SecretRecoveryPhraseBody from 'core/components/SecretRecoveryPhraseBody';
import { useActiveAccount } from 'core/hooks/useAccounts';
import Copyable from 'core/components/Copyable';
import {
  buttonBorderColor, rotationKeyButtonBgColor, customColors,
} from 'core/colors';
import { Routes } from 'core/routes';

interface LocationState {
  hasRotatedKey: boolean;
}

export default function ManageAccountShowRecoveryPhrase() {
  const { colorMode } = useColorMode();
  const { state } = useLocation();
  const { activeAccount } = useActiveAccount();
  const navigate = useNavigate();
  const methods = useForm({
    defaultValues: {
      mnemonic: activeAccount?.mnemonic?.split(' '),
      mnemonicString: activeAccount?.mnemonic,
      showSecretRecoveryPhrase: false,
    },
  });

  const hasRotatedKey = useMemo(() => (state as LocationState)?.hasRotatedKey, [state]);

  return (
    <WalletLayout hasWalletFooter={false} showBackButton>
      <Box width="100%" height="100%" display="flex" flexDirection="column">
        <Box display="flex" width="100%" height="100%" px={4} flex={1}>
          <FormProvider {...methods}>
            <SecretRecoveryPhraseBody />
          </FormProvider>
        </Box>
        <VStack width="100%" spacing={2} borderTop="1px" pt={4} px={4} borderColor={buttonBorderColor[colorMode]}>
          {hasRotatedKey
            ? (
              <Box width="100%" display="flex" gap={2} flexDirection="column">
                <Copyable
                  prompt="Copy secret recovery phrase"
                  value={activeAccount.mnemonic}
                >
                  <Button
                    width="100%"
                    bgColor={rotationKeyButtonBgColor[colorMode]}
                    border="1px"
                    height="48px"
                    borderColor={customColors.navy[200]}
                  >
                    Copy
                  </Button>
                </Copyable>
                <Button
                  width="100%"
                  colorScheme="salmon"
                  height="48px"
                  color="white"
                  onClick={() => navigate(Routes.settings.path)}
                >
                  Back to settings
                </Button>
              </Box>
            )
            : (
              <Box width="100%" display="flex" gap={2} flexDirection="column">
                <Button
                  width="100%"
                  type="submit"
                  onClick={() => navigate(-1)}
                  px={8}
                  py={6}
                  height="48px"
                  border="1px"
                  bgColor={rotationKeyButtonBgColor[colorMode]}
                  borderColor={customColors.navy[200]}
                >
                  Done
                </Button>
              </Box>
            )}
        </VStack>
      </Box>
    </WalletLayout>
  );
}
