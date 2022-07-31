// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box, Button, Flex, Grid, Tooltip, useColorMode,
} from '@chakra-ui/react';
import { Steps, Step } from 'chakra-ui-steps';
import { secondaryBgColor } from 'core/colors';
import { useOnboardingStateContext } from 'core/hooks/useOnboardingState';
import React, { useCallback, useMemo, useState } from 'react';
import { FormProvider, useForm, useFormContext } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';
import * as bip39 from '@scure/bip39';
import { wordlist } from '@scure/bip39/wordlists/english';
import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { passwordOptions } from 'core/components/CreatePasswordBody';
import useWalletState from 'core/hooks/useWalletState';
import { AptosAccount } from 'aptos';
import { generateMnemonicObject } from 'core/utils/account';

zxcvbnOptions.setOptions(passwordOptions);

export enum OnboardingPage {
  CreatePassword = 0,
  SecretRecoveryPhrase = 1,
  Done = 2,
}

const steps = [
  { content: null, label: 'Password' },
  { content: null, label: 'Secret phrase' },
];

export interface OnboardFormValues {
  confirmPassword: string;
  initialPassword: string;
  mnemonic: string[];
  mnemonicString: string;
  secretRecoveryPhrase: boolean;
  termsOfService: boolean;
}

const NextButton = () => {
  const { watch } = useFormContext<OnboardFormValues>();
  const { addAccount } = useWalletState();
  const {
    activeStep, nextStep,
  } = useOnboardingStateContext();
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = useState<boolean>(false);

  const termsOfService = watch('termsOfService');
  const initialPassword = watch('initialPassword');
  const confirmPassword = watch('confirmPassword');
  const secretRecoveryPhrase = watch('secretRecoveryPhrase');
  const mnemonicString = watch('mnemonicString');

  const passwordResult = zxcvbn(initialPassword);
  const passwordScore = passwordResult.score;

  const nextOnClick = useCallback(async () => {
    if (activeStep === 0) {
      nextStep();
    } else if (activeStep === 1) {
      setIsLoading(true);
      const mnemonicObject = await generateMnemonicObject(mnemonicString);
      const aptosAccount = new AptosAccount(mnemonicObject.seed);
      await addAccount({ account: aptosAccount, mnemonic: mnemonicObject });
      setIsLoading(false);
      nextStep();
    } else if (activeStep === 2) {
      navigate(Routes.wallet.routePath);
    }
  }, [activeStep, addAccount, mnemonicString, navigate, nextStep]);

  const NextButtonComponent = useMemo(() => {
    const baseNextButton = (
      <Button isLoading={isLoading} size="md" onClick={nextOnClick} colorScheme="teal">
        {activeStep === steps.length ? 'Finish' : 'Next'}
      </Button>
    );

    const disabledNextButton = (
      <Box>
        <Button isDisabled size="md" onClick={nextOnClick} colorScheme="teal">
          {activeStep === steps.length ? 'Finish' : 'Next'}
        </Button>
      </Box>
    );

    switch (activeStep) {
      case OnboardingPage.CreatePassword: {
        if (termsOfService && initialPassword === confirmPassword && passwordScore > 2) {
          return baseNextButton;
        }
        if (initialPassword !== confirmPassword) {
          return (
            <Tooltip
              label="Passwords must match"
            >
              {disabledNextButton}
            </Tooltip>
          );
        }
        if (passwordScore <= 2) {
          return (
            <Tooltip
              label={'Password strength must be at least "strong"'}
            >
              {disabledNextButton}
            </Tooltip>
          );
        }
        return (
          <Tooltip
            label="You must agree to the Terms of Service"
          >
            {disabledNextButton}
          </Tooltip>
        );
      }
      case OnboardingPage.SecretRecoveryPhrase: {
        if (secretRecoveryPhrase) {
          return baseNextButton;
        }
        return (
          <Tooltip
            label="You must save your Secret Recovery Phrase"
          >
            {disabledNextButton}
          </Tooltip>
        );
      }
      case OnboardingPage.Done: {
        return baseNextButton;
      }
      default: {
        return disabledNextButton;
      }
    }
  }, [
    isLoading,
    nextOnClick,
    activeStep,
    termsOfService,
    initialPassword,
    confirmPassword,
    passwordScore,
    secretRecoveryPhrase,
  ]);

  return NextButtonComponent;
};

const PrevButton = () => {
  const {
    activeStep, prevStep,
  } = useOnboardingStateContext();
  const navigate = useNavigate();

  const prevOnClick = useCallback(() => {
    if (activeStep === 0) {
      navigate(Routes.noWallet.routePath);
    }
    prevStep();
  }, [activeStep, navigate, prevStep]);

  const PrevButtonComponent = useMemo(() => {
    const basePrevButton = (
      <Button
        mr={4}
        onClick={prevOnClick}
        size="md"
        variant="ghost"
      >
        Prev
      </Button>
    );

    return (activeStep > 1) ? null : basePrevButton;
  }, [activeStep, prevOnClick]);

  return PrevButtonComponent;
};

interface CreateWalletLayoutProps {
  children: React.ReactElement;
}

export default function CreateWalletLayout({
  children,
}: CreateWalletLayoutProps) {
  const {
    activeStep,
  } = useOnboardingStateContext();
  const mnemonic = bip39.generateMnemonic(wordlist);
  const methods = useForm<OnboardFormValues>({
    defaultValues: {
      confirmPassword: '',
      initialPassword: '',
      mnemonic: mnemonic.split(' '),
      mnemonicString: mnemonic,
    },
  });

  const { colorMode } = useColorMode();
  return (
    <FormProvider {...methods}>
      <Grid
        height="100%"
        width="100%"
        maxW="100%"
        templateRows="60px 1fr 55px"
        bgColor={secondaryBgColor[colorMode]}
      >
        <Flex px={4}>
          <Steps
            size="sm"
            activeStep={activeStep}
            colorScheme="teal"
            orientation="horizontal"
            responsive={false}
          >
            {steps.map(({ content, label }) => (
              <Step label={label} key={label}>
                {content}
              </Step>
            ))}
          </Steps>
        </Flex>
        <Box px={4} height="100%" width="100%" maxH="100%" overflowY="auto">
          <form>
            {children}
          </form>
        </Box>
        <Flex width="100%" justify="flex-end" px={4} pb={4}>
          <PrevButton />
          <NextButton />
        </Flex>
      </Grid>
    </FormProvider>
  );
}
