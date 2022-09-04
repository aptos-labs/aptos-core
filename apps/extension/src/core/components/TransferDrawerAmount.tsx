// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { secondaryTextColor, secondaryErrorMessageColor, secondaryDividerColor } from 'core/colors';
import {
  Box,
  Button,
  DrawerBody,
  DrawerFooter,
  DrawerHeader,
  Grid,
  HStack,
  Input,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import { useTransferFlow } from 'core/hooks/useTransferFlow';
import TransferAvatar from './TransferAvatar';
import TransferInput from './TransferInput';

export default function TransferDrawerAmount() {
  const { colorMode } = useColorMode();
  const {
    canSubmitForm,
    closeDrawer,
    doesRecipientAccountExist,
    formMethods,
    nextOnClick,
    transferDrawerPage,
    validRecipientAddress,
  } = useTransferFlow();

  const { formState: { isSubmitting }, register } = formMethods;
  const explorerAddress = `https://explorer.devnet.aptos.dev/account/${validRecipientAddress}`;

  return (
    <>
      <DrawerHeader borderBottomWidth="1px" px={4} position="relative">
        <Box
          position="absolute"
          top="0px"
          width="100%"
        >
          <Text
            fontSize="3xl"
            fontWeight={600}
            position="absolute"
            bottom="1rem"
          >
            {transferDrawerPage}
          </Text>
        </Box>
        <HStack spacing={4}>
          <TransferAvatar
            doesRecipientAccountExist={doesRecipientAccountExist}
            recipient={validRecipientAddress}
          />
          <VStack boxSizing="border-box" spacing={0} alignItems="flex-start" flexGrow={1}>
            <Input
              pb={1}
              variant="unstyled"
              size="sm"
              fontWeight={600}
              autoComplete="off"
              spellCheck="false"
              placeholder="Please enter an address"
              {...register('recipient')}
            />
            {doesRecipientAccountExist ? (
              <Button
                color={secondaryTextColor[colorMode]}
                fontSize="sm"
                fontWeight={400}
                height="24px"
                as="a"
                target="_blank"
                rightIcon={<ExternalLinkIcon />}
                variant="unstyled"
                cursor="pointer"
                href={explorerAddress}
                tabIndex={-1}
              >
                View on explorer
              </Button>
            ) : (
              <Button
                color={
                  doesRecipientAccountExist
                    ? secondaryTextColor[colorMode]
                    : secondaryErrorMessageColor[colorMode]
                }
                fontSize="sm"
                fontWeight={400}
                height="24px"
                variant="unstyled"
                cursor="default"
              >
                { validRecipientAddress
                  ? 'Account not found, will be created'
                  : 'Invalid address' }
              </Button>
            )}
          </VStack>
        </HStack>
      </DrawerHeader>
      <DrawerBody px={0} py={0}>
        <TransferInput />
      </DrawerBody>
      <DrawerFooter
        borderTopColor={secondaryDividerColor[colorMode]}
        borderTopWidth="1px"
        px={4}
      >
        <Grid gap={4} width="100%" templateColumns="1fr 1fr">
          <Button onClick={closeDrawer}>
            Cancel
          </Button>
          <Button
            isLoading={isSubmitting}
            isDisabled={!canSubmitForm}
            colorScheme="teal"
            onClick={nextOnClick}
          >
            Next
          </Button>
        </Grid>
      </DrawerFooter>
    </>
  );
}
