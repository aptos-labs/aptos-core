// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Flex,
  IconButton,
  Grid,
  Text,
  useColorMode,
} from '@chakra-ui/react';
import React from 'react';
import {
  ArrowBackIcon,
} from '@chakra-ui/icons';
import {
  secondaryBorderColor,
  secondaryBackButtonBgColor,
} from 'core/colors';
import { useNavigate } from 'react-router-dom';

interface WalletHeaderProps {
  backPage?: string;
  headerValue?: string;
}

export default function ImportAccountHeader({
  backPage,
  headerValue = 'Import wallet',
}: WalletHeaderProps) {
  const { colorMode } = useColorMode();
  const navigate = useNavigate();

  const handleClickBack = () => {
    if (backPage) {
      navigate(backPage);
    }
  };

  return (
    <Grid
      maxW="100%"
      width="100%"
      p={4}
      templateColumns={backPage ? '50px 1fr 40px' : '1fr'}
      borderBottomColor={secondaryBorderColor[colorMode]}
      borderBottomWidth="1px"
      alignItems="center"
    >
      {backPage && (
        <IconButton
          size="lg"
          aria-label="back"
          colorScheme="teal"
          icon={<ArrowBackIcon fontSize={26} />}
          variant="filled"
          onClick={handleClickBack}
          bgColor={secondaryBackButtonBgColor[colorMode]}
          borderRadius="1rem"
        />
      )}
      <Flex width="100%" marginLeft={4}>
        <Text fontSize={20} fontWeight={600}>
          {headerValue}
        </Text>
      </Flex>
      <Box />
    </Grid>
  );
}
