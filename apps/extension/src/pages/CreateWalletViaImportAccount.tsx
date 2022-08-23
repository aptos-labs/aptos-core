// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import { ImportOnboardingStateProvider, useImportOnboardingState } from 'core/hooks/useImportOnboardingState';
import { CreateWalletViaImportLayout, ImportOnboardingPage } from 'core/layouts/CreateWalletViaImportLayout';
import CreatePasswordBody from 'core/components/CreatePasswordBody';
import ConfirmOnboardBody from 'core/components/ConfirmOnboardBody';
import { NoWalletAddAccountBody } from 'core/components/AddAccountBody';
import ImportAccountMnemonicBody from 'core/components/ImportAccountMnemonicBody';
import ImportAccountPrivateKeyBody from 'core/components/ImportAccountPrivateKeyBody';

function NewWalletBody() {
  const { activeStep } = useImportOnboardingState();

  const onboardContent = useMemo(() => {
    switch (activeStep) {
      case ImportOnboardingPage.CreatePassword:
        return <CreatePasswordBody />;
      case ImportOnboardingPage.AddAccount:
        return <NoWalletAddAccountBody px={0} />;
      case ImportOnboardingPage.EnterMnemonic:
        return <ImportAccountMnemonicBody hasSubmit={false} px={0} />;
      case ImportOnboardingPage.EnterPrivateKey:
        return <ImportAccountPrivateKeyBody hasSubmit={false} px={0} />;
      case ImportOnboardingPage.Done:
        return <ConfirmOnboardBody />;
      default:
        return <CreatePasswordBody />;
    }
  }, [activeStep]);

  return onboardContent;
}

function CreateWalletViaImportAccount() {
  return (
    <ImportOnboardingStateProvider>
      <CreateWalletViaImportLayout>
        <NewWalletBody />
      </CreateWalletViaImportLayout>
    </ImportOnboardingStateProvider>
  );
}

export default CreateWalletViaImportAccount;
