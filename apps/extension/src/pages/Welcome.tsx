// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect } from 'react';
import {
  Box,
  Spinner,
  Center,
  Text,
  VStack,
} from '@chakra-ui/react';
import { useNavigate } from 'react-router-dom';
import WaletIcon from 'core/components/WalletIcon';
import Routes from 'core/routes';

function Welcome() {
  const navigate = useNavigate();

  useEffect(() => {
    setTimeout(() => {
      navigate(Routes.wallet.path);
    }, 2000);
  }, [navigate]);

  return (
    <Center height="100%" alignItems="center">
      <VStack width="100%" spacing={2} justifyContent="center">
        <WaletIcon />
        <Text fontSize={28} textAlign="center" fontWeight="bold">Welcome to your wallet</Text>
        <Box>
          <Spinner size="md" />
        </Box>
      </VStack>
    </Center>
  );
}

export default Welcome;
