// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack, Box, Button, Flex, Grid, Tooltip, useColorMode, HStack, IconButton,
} from '@chakra-ui/react';
import { ArrowBackIcon } from '@chakra-ui/icons';
import {
  secondaryBgColor, secondaryBackButtonBgColor, customColors, secondaryButtonBgColor,
} from 'core/colors';
import { useOnboardingState } from 'core/hooks/useOnboardingState';
import React, {
  useCallback, useMemo, useRef, useState,
} from 'react';
import { FormProvider, useForm, useFormContext } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { Transition, type TransitionStatus } from 'react-transition-group';
import Routes from 'core/routes';
import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { passwordOptions } from 'core/components/CreatePasswordBody';
import { AptosAccount } from 'aptos';
import { generateMnemonic, generateMnemonicObject, keysFromAptosAccount } from 'core/utils/account';
import { useAccounts } from 'core/hooks/useAccounts';
import useFundAccount from 'core/mutations/faucet';
import { passwordStrength } from 'core/constants';
import Step from 'core/components/Step';
import SecretPhraseConfirmationPopup from 'core/components/SecretPhraseConfirmationPopup';
import Copyable from '../components/Copyable';

zxcvbnOptions.setOptions(passwordOptions);

export enum OnboardingPage {
  CreatePassword = 0,
  SecretRecoveryPhrase = 1,
  EnterSecretRecoveryPhrase = 2,
}

const steps = [
  { content: null, label: 'Password' },
  { content: null, label: 'Secret Recovery Phrase' },
  { content: null, label: 'Enter Your Secret Recovery Phrase' },
];

export interface CreateWalletFormValues {
  confirmPassword: string;
  confirmPasswordFocused: boolean;
  confirmSavedsecretRecoveryPhrase: boolean;
  initialPassword: string;
  mnemonic: string[];
  mnemonicString: string;
  mnemonicValues: { [key: number]: string },
  savedSecretRecoveryPhrase: boolean;
  showPassword: boolean;
  showSecretRecoveryPhrase: boolean;
  showSecretRecoveryPhrasePopup: boolean;
  termsOfService: boolean;
}

interface NextButtonProps {
  isImport?: boolean;
}

function CopyButton() {
  const { setValue, watch } = useFormContext<CreateWalletFormValues>();
  const { colorMode } = useColorMode();

  const mnemonic = watch('mnemonic');
  const showSecretRecoveryPhrase = watch('showSecretRecoveryPhrase');
  const savedSecretRecoveryPhrase = watch('savedSecretRecoveryPhrase');
  if (!showSecretRecoveryPhrase) return null;

  return (
    <Copyable value={mnemonic.join(' ')} width="100%" copiedPrompt="">
      <Button
        width="100%"
        height="48px"
        size="md"
        border="1px"
        bgColor={secondaryButtonBgColor[colorMode]}
        borderColor={customColors.navy[300]}
        onClick={() => {
          setValue('savedSecretRecoveryPhrase', true);
          setTimeout(() => {
            setValue('savedSecretRecoveryPhrase', false);
          }, 3000);
        }}
      >
        {savedSecretRecoveryPhrase ? 'Copied!' : 'Copy'}
      </Button>
    </Copyable>
  );
}

function NextButton({
  isImport = false,
}: NextButtonProps) {
  const { setValue, watch } = useFormContext<CreateWalletFormValues>();
  const {
    activeStep, nextStep,
  } = useOnboardingState();
  const termsOfService = watch('termsOfService');
  const initialPassword = watch('initialPassword');
  const confirmPassword = watch('confirmPassword');
  const mnemonicString = watch('mnemonicString');
  const mnemonicValues = watch('mnemonicValues');
  const passwordResult = zxcvbn(initialPassword);
  const passwordScore = passwordResult.score;

  const nextOnClick = useCallback(
    async () => {
      if (activeStep === 0 || activeStep === 1) {
        nextStep();
      } else if (activeStep === 2) {
        setValue('showSecretRecoveryPhrasePopup', true);
      }
    },
    [setValue,
      activeStep,
      nextStep,
    ],
  );

  const NextButtonComponent = useMemo(() => {
    const baseNextButton = (
      <Button width="100%" size="md" onClick={nextOnClick} height="48px" colorScheme="salmon">
        Continue
      </Button>
    );

    const disabledNextButton = (
      <Button width="100%" isDisabled size="md" onClick={nextOnClick} colorScheme="salmon" height="48px">
        Continue
      </Button>
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
              <Box width="100%" height="100%">
                {disabledNextButton}
              </Box>
            </Tooltip>
          );
        }
        if (passwordScore < passwordStrength) {
          return (
            <Tooltip
              label={'Password strength must be at least "strong"'}
            >
              <Box width="100%" height="100%">
                {disabledNextButton}
              </Box>
            </Tooltip>
          );
        }
        return (
          <Tooltip
            label="You must agree to the Terms of Service"
          >
            <Box width="100%" height="100%">
              {disabledNextButton}
            </Box>
          </Tooltip>
        );
      }
      case OnboardingPage.SecretRecoveryPhrase: {
        return baseNextButton;
      }
      case OnboardingPage.EnterSecretRecoveryPhrase: {
        const sortedMnemonicEntries = Object.entries(mnemonicValues)
          .sort((a, b) => Number(a[0]) - Number(b[0]));
        const mnemonicInputString = sortedMnemonicEntries.map((v) => v[1]).join(' ');
        if (mnemonicString === mnemonicInputString) {
          return baseNextButton;
        }

        return (
          <Tooltip
            label="You must enter correct Secret Recovery Phrase"
          >
            <Box width="100%" height="100%">
              {disabledNextButton}
            </Box>
          </Tooltip>
        );
      }
      default: {
        return disabledNextButton;
      }
    }
  }, [
    nextOnClick,
    mnemonicValues,
    mnemonicString,
    activeStep,
    termsOfService,
    initialPassword,
    confirmPassword,
    passwordScore,
  ]);

  return (isImport && activeStep >= 1) ? null : NextButtonComponent;
}

const transitionDuration = 200;

interface CreateWalletLayoutProps {
  children: React.ReactElement;
}

const buttonBorderColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

function CreateWalletLayout({
  children,
}: CreateWalletLayoutProps) {
  const { setValue, watch } = useFormContext<CreateWalletFormValues>();
  const { initAccounts } = useAccounts();
  const { fundAccount } = useFundAccount();
  const [loading, setLoading] = useState<boolean>(false);

  const {
    activeStep, nextStep, prevStep,
  } = useOnboardingState();
  const navigate = useNavigate();
  const confirmPassword = watch('confirmPassword');
  const mnemonicString = watch('mnemonicString');
  const showSecretRecoveryPhrase = watch('showSecretRecoveryPhrase');
  const showSecretRecoveryPhrasePopup = watch('showSecretRecoveryPhrasePopup');
  const ref = useRef(null);

  const termsOfService = watch('termsOfService');
  const initialPassword = watch('initialPassword');
  const savedSecretRecoveryPhrase = watch('savedSecretRecoveryPhrase');

  const passwordResult = zxcvbn(initialPassword);
  const passwordScore = passwordResult.score;

  const initAccount = async () => {
    setLoading(true);
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
    setLoading(false);
  };

  const nextOnClick = useCallback(async () => {
    if (activeStep === 0) {
      if (termsOfService
        && initialPassword === confirmPassword
         && passwordScore >= passwordStrength) {
        nextStep();
      }
    } else if (activeStep === 1) {
      if (savedSecretRecoveryPhrase) {
        nextStep();
      }
    } else if (activeStep === 3) {
      navigate(Routes.wallet.path);
    }
  }, [
    initialPassword,
    activeStep,
    confirmPassword,
    passwordScore,
    savedSecretRecoveryPhrase,
    termsOfService,
    navigate,
    nextStep,
  ]);

  const prevOnClick = useCallback(() => {
    if (activeStep === 0) {
      navigate(Routes.noWallet.path);
    }
    prevStep();
  }, [activeStep, navigate, prevStep]);

  const { colorMode } = useColorMode();

  return (
    <Grid
      height="100%"
      width="100%"
      maxW="100%"
      templateRows={`60px 1fr ${showSecretRecoveryPhrase ? 132 : 84}px`}
      bgColor={secondaryBgColor[colorMode]}
      position="relative"
    >
      <HStack width="100%" px={4}>
        <IconButton
          position="absolute"
          size="md"
          aria-label="back"
          colorScheme="teal"
          icon={<ArrowBackIcon fontSize={20} />}
          variant="filled"
          onClick={prevOnClick}
          bgColor={secondaryBackButtonBgColor[colorMode]}
          borderRadius="1rem"
        />
        <Flex justifyContent="center" width="100%">
          <HStack spacing="0" justify="space-evenly" width="40%">
            {steps.map(({ label }, id) => (
              <Step
                key={label}
                cursor="pointer"
                onClick={activeStep > id ? prevOnClick : nextOnClick}
                isActive={activeStep === id}
                isCompleted={activeStep > id}
                isLastStep={id === steps.length - 1}
              />
            ))}
          </HStack>
        </Flex>
      </HStack>
      <Box px={4} height="100%" width="100%" maxH="100%" overflowY="auto">
        <form>
          {children}
        </form>
      </Box>
      <Flex width="100%" justify="flex-end" alignItems="center">
        <VStack width="full" borderTop="1px" py={4} borderColor={buttonBorderColor[colorMode]}>
          <Flex width="100%" px={4} gap={2} flexDirection="column">
            <CopyButton />
            <NextButton />
          </Flex>
        </VStack>
      </Flex>
      <Transition in={showSecretRecoveryPhrasePopup} timeout={transitionDuration} nodeRef={ref}>
        {(state: TransitionStatus) => (
          <SecretPhraseConfirmationPopup
            open={showSecretRecoveryPhrasePopup}
            duration={transitionDuration}
            state={state}
            isLoading={loading}
            goPrev={() => {
              setValue('showSecretRecoveryPhrasePopup', false);
            }}
            goNext={async () => {
              await initAccount();
              navigate(Routes.welcome.path);
            }}
          />
        )}
      </Transition>
    </Grid>
  );
}

export default function CreateWalletLayoutContainer(props: any) {
  const mnemonic = generateMnemonic();
  const methods = useForm<CreateWalletFormValues>({
    defaultValues: {
      confirmPassword: '',
      confirmPasswordFocused: false,
      confirmSavedsecretRecoveryPhrase: false,
      initialPassword: '',
      mnemonic: mnemonic.split(' '),
      mnemonicString: mnemonic,
      mnemonicValues: {},
      savedSecretRecoveryPhrase: false,
      showPassword: false,
      showSecretRecoveryPhrase: false,
      showSecretRecoveryPhrasePopup: false,
    },
  });

  return (
    <FormProvider {...methods}>
      <CreateWalletLayout {...props} />
    </FormProvider>
  );
}
