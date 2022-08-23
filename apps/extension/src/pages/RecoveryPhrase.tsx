// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import RecoveryPhraseBox from 'core/components/RecoveryPhraseBox';

function RecoveryPhrase() {
  return (
    <WalletLayout title="Recovery Phrase" showBackButton>
      <VStack width="100%" height="100%" spacing={8} paddingTop={8} paddingStart={8} paddingEnd={8}>
        <RecoveryPhraseBox />
      </VStack>
    </WalletLayout>
  );
}

export default RecoveryPhrase;
