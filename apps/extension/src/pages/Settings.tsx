// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  useColorMode,
  VStack,
  Flex,
  SimpleGrid,
  Tooltip,
  useClipboard,
  Text,
} from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import SettingsPaths from 'core/components/SettingsPaths';
import SettingsListItem from 'core/components/SettingsListItem';
import { secondaryTextColor } from 'core/colors';
import { CredentialHeaderAndBodyProps } from 'core/components/CredentialsBody';
import { useActiveAccount } from 'core/hooks/useAccounts';

export function CredentialRow({
  body,
  header,
}: CredentialHeaderAndBodyProps) {
  const { hasCopied, onCopy } = useClipboard(body || '');
  const { colorMode } = useColorMode();
  return (
    <SimpleGrid columns={2} width="100%">
      <Flex alignItems="flex-start">
        <Text fontSize="md" color={secondaryTextColor[colorMode]}>
          {header}
        </Text>
      </Flex>
      <Flex alignItems="flex-end">
        <Tooltip label={hasCopied ? 'Copied!' : 'Copy'} closeDelay={300}>
          <Text fontSize="md" cursor="pointer" noOfLines={1} onClick={onCopy}>
            {body}
          </Text>
        </Tooltip>
      </Flex>
    </SimpleGrid>
  );
}

function Account() {
  const { activeAccount } = useActiveAccount();
  const mnemonic = activeAccount?.mnemonic;

  return (
    <WalletLayout title="Settings">
      <VStack width="100%" paddingTop={8} px={4} pb={4} spacing={2}>
        {
          SettingsPaths(mnemonic !== undefined).map((value) => (
            <SettingsListItem
              key={value.path}
              {...value}
            />
          ))
        }
      </VStack>
    </WalletLayout>
  );
}

export default Account;
