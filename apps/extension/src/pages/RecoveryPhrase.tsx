// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import RecoveryPhraseBox from 'core/components/RecoveryPhraseBox';

function RecoveryPhrase() {
  return (
    <AuthLayout routePath={PageRoutes.recovery_phrase.path}>
      <WalletLayout title="Recovery Phrase" showBackButton>
        <VStack width="100%" height="100%" spacing={8} paddingTop={8} paddingStart={8} paddingEnd={8}>
          <RecoveryPhraseBox />
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default RecoveryPhrase;
