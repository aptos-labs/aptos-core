// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  useColorMode,
  VStack,
  Button,
  Flex,
  SimpleGrid,
  Tooltip,
  useClipboard,
  Text,
} from '@chakra-ui/react';
import React from 'react';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import WalletLayout from 'core/layouts/WalletLayout';
import SettingsPaths from 'core/components/SettingsPaths';
import SettingsListItem from 'core/components/SettingsListItem';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import { secondaryTextColor } from 'core/colors';
import { CredentialHeaderAndBodyProps } from 'core/components/CredentialsBody';
import useGlobalStateContext from 'core/hooks/useGlobalState';

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
  const { activeAccount } = useGlobalStateContext();
  const address = activeAccount?.address;
  const mnemonic = activeAccount?.mnemonic;

  const explorerAddress = address ? `https://explorer.devnet.aptos.dev/account/${address}` : 'https://explorer.devnet.aptos.dev';

  return (
    <AuthLayout routePath={PageRoutes.settings.path}>
      <WalletLayout title="Settings">
        <VStack width="100%" paddingTop={8}>
          <Box px={4} pb={4} width="100%">
            <Flex pb={2} pt={1} justifyContent="end">
              <Button
                fontSize="sm"
                fontWeight={400}
                as="a"
                target="_blank"
                rightIcon={<ExternalLinkIcon />}
                variant="unstyled"
                cursor="pointer"
                href={explorerAddress}
              >
                View on explorer
              </Button>
            </Flex>
            <VStack spacing={2}>
              {
              SettingsPaths(mnemonic !== undefined).map((value) => (
                <SettingsListItem
                  key={value.path}
                  {...value}
                />
              ))
            }
            </VStack>
          </Box>
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default Account;
