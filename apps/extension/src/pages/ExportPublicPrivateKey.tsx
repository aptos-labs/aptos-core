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
} from '@chakra-ui/react';
import { RiErrorWarningFill } from '@react-icons/all-files/ri/RiErrorWarningFill';
import { RiFileCopyLine } from '@react-icons/all-files/ri/RiFileCopyLine';
import { secondaryTextColor } from 'core/colors';
import Copyable from 'core/components/Copyable';
import WalletLayout from 'core/layouts/WalletLayout';
import { useActiveAccount } from 'core/hooks/useAccounts';

export default function ExportPublicPrivateKey() {
  const { colorMode } = useColorMode();
  const { activeAccount } = useActiveAccount();
  return (
    <WalletLayout
      title="Export Keys"
      showBackButton
    >
      <VStack width="100%" paddingTop={4}>
        <Flex px={4} pb={4} width="100%" flexDirection="column" gap={8}>
          <Box width="100%">
            <Flex>
              <Text
                fontSize={18}
                fontWeight={500}
                flex={1}
              >
                Public Key
              </Text>
              <Copyable
                prompt="Copy public key"
                value={activeAccount.publicKey}
              >
                <HStack alignItems="baseline" gap={2} color="teal">
                  <Box margin="auto">
                    <RiFileCopyLine size={20} />
                  </Box>
                  <Text
                    fontSize={18}
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
              value={activeAccount.publicKey}
            />
          </Box>
          <Box width="100%">
            <Flex>
              <Text
                fontSize={18}
                fontWeight={500}
                flex={1}
              >
                Private Key
              </Text>
              <Copyable
                prompt="Copy private key"
                value={activeAccount.privateKey}
              >
                <HStack alignItems="baseline" gap={2} color="teal">
                  <Box margin="auto">
                    <RiFileCopyLine size={20} />
                  </Box>
                  <Text
                    fontSize={18}
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
          <Flex width="100%" px={4} bgColor="#C46B021A" p={4}>
            <Flex width="60px" justifyContent="center" alignItems="baseline">
              <RiErrorWarningFill color="#D76D61" size={24} />
            </Flex>
            <VStack textAlign="left">
              <Text fontSize="md" fontWeight={700} marginRight="auto">Don&apos;t share your private key</Text>
              <Text fontSize="md">Anyone with your key has full access to your assets</Text>
            </VStack>
          </Flex>
        </Flex>
      </VStack>
    </WalletLayout>
  );
}
