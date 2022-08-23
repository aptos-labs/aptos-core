// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import constate from 'constate';
import { ImportOnboardingPage } from 'core/layouts/CreateWalletViaImportLayout';
import Routes from 'core/routes';
import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';

export default function useImportOnboardingStateRecorder() {
  const [
    activeStep,
    setActiveStep,
  ] = useState<ImportOnboardingPage>(ImportOnboardingPage.CreatePassword);

  const navigate = useNavigate();

  const nextStep = useCallback(() => {
    switch (activeStep) {
      case ImportOnboardingPage.CreatePassword:
        setActiveStep(ImportOnboardingPage.AddAccount);
        break;
      case ImportOnboardingPage.AddAccount:
        throw new Error('Please decide between import mnemonic and import private key');
      case ImportOnboardingPage.EnterMnemonic:
        setActiveStep(ImportOnboardingPage.Done);
        break;
      case ImportOnboardingPage.EnterPrivateKey:
        setActiveStep(ImportOnboardingPage.Done);
        break;
      case ImportOnboardingPage.Done:
        navigate(Routes.wallet.path);
        break;
      default:
        throw new Error('Undefined step');
    }
    setActiveStep(activeStep + 1);
  }, [activeStep, navigate]);

  const prevStep = useCallback(() => {
    switch (activeStep) {
      case ImportOnboardingPage.AddAccount:
        setActiveStep(ImportOnboardingPage.CreatePassword);
        break;
      case ImportOnboardingPage.EnterMnemonic:
      case ImportOnboardingPage.EnterPrivateKey:
        setActiveStep(ImportOnboardingPage.AddAccount);
        break;
      default:
        setActiveStep(ImportOnboardingPage.CreatePassword);
    }
  }, [activeStep]);

  return {
    activeStep,
    nextStep,
    prevStep,
    setActiveStep,
  };
}

export const [
  ImportOnboardingStateProvider,
  useImportOnboardingState,
] = constate(useImportOnboardingStateRecorder);
