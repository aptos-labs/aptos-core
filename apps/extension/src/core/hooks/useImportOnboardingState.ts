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
  ] = useState<ImportOnboardingPage>(ImportOnboardingPage.ImportType);

  const navigate = useNavigate();

  const nextStep = useCallback(() => {
    switch (activeStep) {
      case ImportOnboardingPage.ImportType:
        setActiveStep(ImportOnboardingPage.CreatePassword);
        break;
      case ImportOnboardingPage.CreatePassword:
        setActiveStep(ImportOnboardingPage.Done);
        break;
      case ImportOnboardingPage.ImportMnemonic:
        setActiveStep(ImportOnboardingPage.CreatePassword);
        break;
      case ImportOnboardingPage.ImportPrivateKey:
        setActiveStep(ImportOnboardingPage.CreatePassword);
        break;
      case ImportOnboardingPage.Done:
        navigate(Routes.wallet.path);
        break;
      default:
        throw new Error('Undefined step');
    }
  }, [activeStep, navigate]);

  const prevStep = useCallback(() => {
    switch (activeStep) {
      case ImportOnboardingPage.ImportMnemonic:
      case ImportOnboardingPage.ImportMnemonicOrPrivateKey:
      case ImportOnboardingPage.CreatePassword:
        setActiveStep(ImportOnboardingPage.ImportType);
        break;

      default:
        setActiveStep(ImportOnboardingPage.ImportType);
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
