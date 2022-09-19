// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import { OnboardingStateProvider, useOnboardingState } from 'core/hooks/useOnboardingState';
import CreateWalletLayout, { OnboardingPage } from 'core/layouts/CreateWalletLayout';
import CreatePasswordBody from 'core/components/CreatePasswordBody';
import SecretRecoveryPhraseBody from 'core/components/SecretRecoveryPhraseBody';
import EnterSecretRecoveryPhraseBody from 'core/components/EnterSecretRecoveryPhraseBody';

function NewWalletBody() {
  const { activeStep } = useOnboardingState();

  const onboardContent = useMemo(() => {
    switch (activeStep) {
      case OnboardingPage.CreatePassword:
        return <CreatePasswordBody />;
      case OnboardingPage.SecretRecoveryPhrase:
        return <SecretRecoveryPhraseBody inputHeight={42} />;
      case OnboardingPage.EnterSecretRecoveryPhrase:
        return <EnterSecretRecoveryPhraseBody />;
      default:
        return <CreatePasswordBody />;
    }
  }, [activeStep]);

  return onboardContent;
}

function CreateWallet() {
  return (
    <OnboardingStateProvider>
      <CreateWalletLayout>
        <NewWalletBody />
      </CreateWalletLayout>
    </OnboardingStateProvider>
  );
}

export default CreateWallet;
