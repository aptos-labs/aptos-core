// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import { OnboardingStateProvider, useOnboardingState } from 'core/hooks/useOnboardingState';
import CreateWalletLayout, { OnboardingPage } from 'core/layouts/CreateWalletLayout';
import CreatePasswordBody from 'core/components/CreatePasswordBody';
import SecretRecoveryPhraseBody from 'core/components/SecretRecoveryPhraseBody';
import ConfirmOnboardBody from 'core/components/ConfirmOnboardBody';

function NewWalletBody() {
  const { activeStep } = useOnboardingState();

  const onboardContent = useMemo(() => {
    switch (activeStep) {
      case OnboardingPage.CreatePassword:
        return <CreatePasswordBody />;
      case OnboardingPage.SecretRecoveryPhrase:
        return <SecretRecoveryPhraseBody />;
      case OnboardingPage.Done:
        return <ConfirmOnboardBody />;
      default:
        return <CreatePasswordBody />;
    }
  }, [activeStep]);

  return onboardContent;
}

function CreateWallet() {
  return (
    <AuthLayout routePath={PageRoutes.createWallet.path}>
      <OnboardingStateProvider>
        <CreateWalletLayout>
          <NewWalletBody />
        </CreateWalletLayout>
      </OnboardingStateProvider>
    </AuthLayout>
  );
}

export default CreateWallet;
