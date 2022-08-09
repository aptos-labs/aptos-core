// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  HStack,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import WalletAccountBalance from 'core/components/WalletAccountBalance';
import TransferDrawer from 'core/components/TransferDrawer';
import Faucet from 'core/components/Faucet';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import useGlobalStateContext from 'core/hooks/useGlobalState';

function Wallet() {
  const { faucetClient } = useGlobalStateContext();

  return (
    <AuthLayout routePath={PageRoutes.wallet.routePath}>
      <WalletLayout>
        <VStack width="100%" paddingTop={8}>
          <WalletAccountBalance />
          <HStack spacing={4}>
            { faucetClient && <Faucet /> }
            <TransferDrawer />
          </HStack>
        </VStack>
      </WalletLayout>
    </AuthLayout>

  );
}

export default Wallet;
