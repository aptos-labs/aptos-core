// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useMemo } from 'react';
import { ImportOnboardingStateProvider, useImportOnboardingState } from 'core/hooks/useImportOnboardingState';
import { CreateWalletViaImportLayout, ImportOnboardingPage } from 'core/layouts/CreateWalletViaImportLayout';
import CreatePasswordBody from 'core/components/CreatePasswordBody';
import { NoWalletAddAccountBody } from 'core/components/AddAccountBody';
import ImportAccountMnemonicBody from 'core/components/ImportAccountMnemonicBody';
import ImportAccountPrivateKeyBody from 'core/components/ImportAccountPrivateKeyBody';
import { useNavigate } from 'react-router-dom';
import { Routes } from 'core/routes';

function NewWalletBody() {
  const { activeStep } = useImportOnboardingState();
  const navigate = useNavigate();

  useEffect(() => {
    if (activeStep === ImportOnboardingPage.Done) {
      navigate(Routes.welcome.path);
    }
  }, [navigate, activeStep]);

  const onboardContent = useMemo(() => {
    switch (activeStep) {
      case ImportOnboardingPage.ImportType:
        return <NoWalletAddAccountBody px={0} />;
      case ImportOnboardingPage.ImportMnemonic:
        return <ImportAccountMnemonicBody hasSubmit={false} px={0} />;
      case ImportOnboardingPage.ImportPrivateKey:
        return <ImportAccountPrivateKeyBody hasSubmit={false} px={0} />;
      case ImportOnboardingPage.CreatePassword:
        return <CreatePasswordBody />;
      case ImportOnboardingPage.Done:
        return null;
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
