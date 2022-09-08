// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box, Button, Flex, Grid, Tooltip, useColorMode,
} from '@chakra-ui/react';
import { Steps, Step } from 'chakra-ui-steps';
import { secondaryBgColor } from 'core/colors';
import { useOnboardingState } from 'core/hooks/useOnboardingState';
import React, { useCallback, useMemo, useState } from 'react';
import { FormProvider, useForm, useFormContext } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';
import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { passwordOptions } from 'core/components/CreatePasswordBody';
import { AptosAccount } from 'aptos';
import { generateMnemonic, generateMnemonicObject, keysFromAptosAccount } from 'core/utils/account';
import { useAccounts } from 'core/hooks/useAccounts';
import useFundAccount from 'core/mutations/faucet';
import { passwordStrength } from 'core/constants';

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

export interface CreateWalletFormValues {
  confirmPassword: string;
  initialPassword: string;
  mnemonic: string[];
  mnemonicString: string;
  secretRecoveryPhrase: boolean;
  termsOfService: boolean;
}

interface NextButtonProps {
  isImport?: boolean;
}

function NextButton({
  isImport = false,
}: NextButtonProps) {
  const { watch } = useFormContext<CreateWalletFormValues>();
  const { initAccounts } = useAccounts();
  const { fundAccount } = useFundAccount();

  const {
    activeStep, nextStep,
  } = useOnboardingState();
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
      const { mnemonic, seed } = await generateMnemonicObject(mnemonicString);
      const aptosAccount = new AptosAccount(seed);

      const firstAccount = {
        mnemonic,
        ...keysFromAptosAccount(aptosAccount),
      };

      await initAccounts(confirmPassword, {
        [firstAccount.address]: firstAccount,
      });

      if (fundAccount) {
        await fundAccount({ address: firstAccount.address, amount: 0 });
      }

      setIsLoading(false);
      nextStep();
    } else if (activeStep === 2) {
      navigate(Routes.wallet.path);
    }
  }, [activeStep, initAccounts, confirmPassword, fundAccount, mnemonicString, navigate, nextStep]);

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
        if (termsOfService
          && initialPassword === confirmPassword
           && passwordScore >= passwordStrength) {
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
        if (passwordScore < passwordStrength) {
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

  return (isImport && activeStep >= 1) ? null : NextButtonComponent;
}

const PrevButton = () => {
  const {
    activeStep, prevStep,
  } = useOnboardingState();
  const navigate = useNavigate();

  const prevOnClick = useCallback(() => {
    if (activeStep === 0) {
      navigate(Routes.noWallet.path);
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
  } = useOnboardingState();
  const mnemonic = generateMnemonic();
  const methods = useForm<CreateWalletFormValues>({
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
