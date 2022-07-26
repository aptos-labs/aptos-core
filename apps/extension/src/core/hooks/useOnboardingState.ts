// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import constate from 'constate';
import { useState } from 'react';

export default function useOnboardingState() {
  const [activeStep, setActiveStep] = useState<number>(0);

  const nextStep = () => {
    setActiveStep(activeStep + 1);
  };

  const prevStep = () => {
    setActiveStep(activeStep - 1);
  };

  return {
    activeStep,
    nextStep,
    prevStep,
  };
}

export const [OnboardingStateProvider, useOnboardingStateContext] = constate(useOnboardingState);
