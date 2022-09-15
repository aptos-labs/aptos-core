// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  DrawerHeader,
  Box,
  HStack,
  Fade,
  DrawerBody,
  DrawerFooter,
  Grid,
  Button,
  Text,
  useColorMode,
} from '@chakra-ui/react';
import { secondaryDividerColor } from 'core/colors';
import { transferAptFormId, TransferDrawerPage, useTransferFlow } from 'core/hooks/useTransferFlow';
import React from 'react';
import TransferSummary from './TransferSummary';

export default function TransferDrawerConfirm() {
  const { colorMode } = useColorMode();
  const {
    backOnClick,
    canSubmitForm,
    formMethods,
    transferDrawerPage,
  } = useTransferFlow();

  const { formState: { isSubmitting } } = formMethods;

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
            color="white"
          >
            {transferDrawerPage}
          </Text>
        </Box>
        <HStack spacing={4}>
          <Fade in={transferDrawerPage === TransferDrawerPage.CONFIRM_TRANSACTION}>
            <Text>Summary</Text>
          </Fade>
        </HStack>
      </DrawerHeader>
      <DrawerBody px={0} py={0}>
        <TransferSummary />
      </DrawerBody>
      <DrawerFooter
        borderTopColor={secondaryDividerColor[colorMode]}
        borderTopWidth="1px"
        px={4}
      >
        <Grid gap={4} width="100%" templateColumns="1fr 1fr">
          <Button onClick={backOnClick}>
            Back
          </Button>
          <Button
            isLoading={isSubmitting}
            isDisabled={!canSubmitForm}
            colorScheme="teal"
            type="submit"
            form={transferAptFormId}
          >
            Send
          </Button>
        </Grid>
      </DrawerFooter>
    </>
  );
}
