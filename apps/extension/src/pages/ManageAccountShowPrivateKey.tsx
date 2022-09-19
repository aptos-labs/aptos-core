// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  VStack,
  Text,
  Flex,
  useColorMode,
  HStack,
  Box,
  Textarea,
  Icon,
} from '@chakra-ui/react';
import { RiFileCopyLine } from '@react-icons/all-files/ri/RiFileCopyLine';
import { secondaryTextColor } from 'core/colors';
import Copyable from 'core/components/Copyable';
import WalletLayout from 'core/layouts/WalletLayout';
import { useActiveAccount } from 'core/hooks/useAccounts';

export default function ManageAccountShowPrivateKey() {
  const { colorMode } = useColorMode();
  const { activeAccount } = useActiveAccount();
  return (
    <WalletLayout
      title="Show Private Key"
      showBackButton
    >
      <VStack width="100%" paddingTop={4} height="100%">
        <Flex px={4} pb={4} width="100%" flexDirection="column" gap={8} height="100%">
          <Box width="100%" flex={1}>
            <Flex>
              <Text
                fontSize="lg"
                fontWeight={700}
                flex={1}
              >
                Private Key
              </Text>
              <Copyable
                prompt="Copy private key"
                value={activeAccount.privateKey}
              >
                <HStack alignItems="baseline">
                  <Box margin="auto">
                    <Icon as={RiFileCopyLine} my="auto" w={4} h={4} margin="auto" />
                  </Box>
                  <Text
                    fontSize="sm"
                    fontWeight={500}
                  >
                    Copy
                  </Text>
                </HStack>
              </Copyable>
            </Flex>
            <Textarea
              marginTop={4}
              color={secondaryTextColor[colorMode]}
              height={24}
              readOnly
              variant="filled"
              fontSize={18}
              value={activeAccount.privateKey}
            />
          </Box>
        </Flex>
      </VStack>
    </WalletLayout>
  );
}
