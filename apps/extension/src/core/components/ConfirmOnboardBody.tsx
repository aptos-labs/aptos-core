// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
  Center,
  Heading,
  HStack,
  Text,
  VStack,
} from '@chakra-ui/react';
import { IoIosDocument } from '@react-icons/all-files/io/IoIosDocument';
import { FaDiscord } from '@react-icons/all-files/fa/FaDiscord';
import React from 'react';

const aptosDevWalletExtensionUrl = 'https://petra.app/docs/petra-intro';
const aptosWalletDiscordUrl = 'https://discord.com/invite/petrawallet';

export default function ConfirmOnboardBody() {
  return (
    <VStack pt={20}>
      <Heading fontSize="3xl">🎉 You&apos;re all done!</Heading>
      <Text
        fontSize="md"
        textAlign="center"
      >
        Follow us on social media or check out our docs for more info
      </Text>
      <Center>
        <HStack pt={8} spacing={4} width="100%">
          <Button
            as="a"
            href={aptosWalletDiscordUrl}
            target="_blank"
            leftIcon={<FaDiscord />}
            colorScheme="purple"
            bgColor="#7289da"
          >
            Discord
          </Button>
          <Button
            as="a"
            href={aptosDevWalletExtensionUrl}
            target="_blank"
            leftIcon={<IoIosDocument />}
            colorScheme="blue"
          >
            Docs
          </Button>
        </HStack>
      </Center>
    </VStack>
  );
}
