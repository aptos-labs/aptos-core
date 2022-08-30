// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box, Button, Flex, Grid, Tooltip, useColorMode,
} from '@chakra-ui/react';
import { Steps, Step } from 'chakra-ui-steps';
import { secondaryBgColor } from 'core/colors';
import { useImportOnboardingState } from 'core/hooks/useImportOnboardingState';
import React, { useCallback, useMemo, useState } from 'react';
import { FormProvider, useForm, useFormContext } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';
import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { passwordOptions } from 'core/components/CreatePasswordBody';
import { generateMnemonic, generateMnemonicObject } from 'core/utils/account';
import { AptosAccount } from 'aptos';
import {
  importAccountErrorToast, importAccountToast, networkDoesNotExistToast,
} from 'core/components/Toast';
import { useAccounts } from 'core/hooks/useAccounts';
import { useNetworks } from 'core/hooks/useNetworks';
import { MnemonicFormValues } from './AddAccountLayout';

zxcvbnOptions.setOptions(passwordOptions);

export enum ImportOnboardingPage {
  CreatePassword,
  AddAccount,
  EnterMnemonic,
  EnterPrivateKey,
  Done,
}

const ImportOnboardingPageEnumDict = {
  [ImportOnboardingPage.CreatePassword]: 0,
  [ImportOnboardingPage.AddAccount]: 1,
  [ImportOnboardingPage.EnterMnemonic]: 2,
  [ImportOnboardingPage.EnterPrivateKey]: 2,
  [ImportOnboardingPage.Done]: 3,
};

const createViaImportSteps = [
  { content: null, label: 'Password' },
  { content: null, label: 'Import type' },
  { content: null, label: 'Secret key' },
];

export interface CreateWalletViaImportFormValues {
  confirmPassword: string;
  importType: 'privateKey' | 'mnemonic';
  initialPassword: string;
  mnemonic: string[];
  mnemonicString: string;
  privateKey: string;
  secretRecoveryPhrase: boolean;
  termsOfService: boolean;
}

export type CreateWalletViaImportGeneralFormValues =
CreateWalletViaImportFormValues & MnemonicFormValues;

function NextButton() {
  const { watch } = useFormContext<CreateWalletViaImportGeneralFormValues>();
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const { activeNetwork } = useNetworks();

  const {
    initAccounts,
  } = useAccounts();

  const {
    activeStep, nextStep, setActiveStep,
  } = useImportOnboardingState();
  const navigate = useNavigate();

  const termsOfService = watch('termsOfService');
  const initialPassword = watch('initialPassword');
  const confirmPassword = watch('confirmPassword');
  const privateKey = watch('privateKey');
  const importType = watch('importType');
  const allFields = watch();

  const mnemonicArray = useMemo(() => [
    allFields['mnemonic-a'].trim(),
    allFields['mnemonic-b'].trim(),
    allFields['mnemonic-c'].trim(),
    allFields['mnemonic-d'].trim(),
    allFields['mnemonic-e'].trim(),
    allFields['mnemonic-f'].trim(),
    allFields['mnemonic-g'].trim(),
    allFields['mnemonic-h'].trim(),
    allFields['mnemonic-i'].trim(),
    allFields['mnemonic-j'].trim(),
    allFields['mnemonic-k'].trim(),
    allFields['mnemonic-l'].trim(),
  ], [allFields]);

  const passwordResult = zxcvbn(initialPassword);
  const passwordScore = passwordResult.score;

  const nextOnClick = useCallback(async () => {
    switch (activeStep) {
      case ImportOnboardingPage.CreatePassword:
        nextStep();
        return;
      case ImportOnboardingPage.AddAccount: {
        if (allFields.importType === 'mnemonic') {
          setActiveStep(ImportOnboardingPage.EnterMnemonic);
        } else if (allFields.importType === 'privateKey') {
          setActiveStep(ImportOnboardingPage.EnterPrivateKey);
        }

        return;
      }
      case ImportOnboardingPage.EnterMnemonic: {
        try {
          const nodeUrl = activeNetwork?.nodeUrl;
          if (!nodeUrl) {
            networkDoesNotExistToast();
            return;
          }
          setIsLoading(true);
          let mnemonicString = '';
          mnemonicArray.forEach((value) => {
            mnemonicString = `${mnemonicString + value} `;
          });
          mnemonicString = mnemonicString.trim();
          const { mnemonic, seed } = await generateMnemonicObject(mnemonicString);
          const aptosAccount = new AptosAccount(seed);
          const {
            address,
            privateKeyHex,
            publicKeyHex,
          } = aptosAccount.toPrivateKeyObject();

          // initialize password and wallet
          const firstAccount = {
            address: address!,
            mnemonic,
            name: 'Wallet',
            privateKey: privateKeyHex,
            publicKey: publicKeyHex!,
          };

          await initAccounts(confirmPassword, {
            [firstAccount.address]: firstAccount,
          });

          setIsLoading(false);
          importAccountToast();
          nextStep();
        } catch (err) {
          setIsLoading(false);
          importAccountErrorToast();
        }

        return;
      }
      case ImportOnboardingPage.EnterPrivateKey: {
        try {
          const nodeUrl = activeNetwork?.nodeUrl;
          if (!nodeUrl) {
            networkDoesNotExistToast();
            return;
          }
          setIsLoading(true);
          const nonHexKey = (privateKey.startsWith('0x')) ? privateKey.substring(2) : privateKey;
          const encodedKey = Uint8Array.from(Buffer.from(nonHexKey, 'hex'));
          const aptosAccount = new AptosAccount(encodedKey);

          const {
            address,
            privateKeyHex,
            publicKeyHex,
          } = aptosAccount.toPrivateKeyObject();

          // initialize password and wallet
          const firstAccount = {
            address: address!,
            name: 'Wallet',
            privateKey: privateKeyHex,
            publicKey: publicKeyHex!,
          };

          await initAccounts(confirmPassword, {
            [firstAccount.address]: firstAccount,
          });

          setIsLoading(false);
          importAccountToast();
          nextStep();
        } catch (err) {
          setIsLoading(false);
          importAccountErrorToast();
        }

        return;
      }
      case ImportOnboardingPage.Done:
        navigate(Routes.wallet.path);
        return;
      default:
        throw new Error('Undefined next step');
    }
  }, [
    activeNetwork?.nodeUrl,
    activeStep,
    allFields.importType,
    confirmPassword,
    initAccounts,
    mnemonicArray,
    navigate,
    nextStep,
    privateKey,
    setActiveStep,
  ]);

  const NextButtonComponent = useMemo(() => {
    const baseNextButton = (
      <Button isLoading={isLoading} size="md" onClick={nextOnClick} colorScheme="teal">
        {activeStep === ImportOnboardingPage.Done ? 'Finish' : 'Next'}
      </Button>
    );

    const disabledNextButton = (
      <Box>
        <Button isLoading={isLoading} isDisabled size="md" onClick={nextOnClick} colorScheme="teal">
          {activeStep === ImportOnboardingPage.Done ? 'Finish' : 'Next'}
        </Button>
      </Box>
    );

    switch (activeStep) {
      case ImportOnboardingPage.CreatePassword: {
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
      case ImportOnboardingPage.AddAccount: {
        if (importType) {
          return baseNextButton;
        }
        return (
          <Tooltip label="You must choose from the options above">
            {disabledNextButton}
          </Tooltip>
        );
      }
      case ImportOnboardingPage.EnterMnemonic: {
        let allIsFilledIn = true;
        mnemonicArray.forEach((word) => {
          if (word.length === 0) {
            allIsFilledIn = false;
          }
        });
        if (allIsFilledIn) {
          return baseNextButton;
        }
        return (
          <Tooltip label="Please enter all spaces for mnemonic">
            {disabledNextButton}
          </Tooltip>
        );
      }
      case ImportOnboardingPage.EnterPrivateKey: {
        if (!(privateKey.length >= 64 && privateKey.length <= 68)) {
          return (
            <Tooltip label="Please enter a valid private key">
              {disabledNextButton}
            </Tooltip>
          );
        }
        return baseNextButton;
      }
      case ImportOnboardingPage.Done: {
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
    importType,
    mnemonicArray,
    privateKey.length,
  ]);

  return NextButtonComponent;
}

function PrevButton() {
  const {
    activeStep, prevStep,
  } = useImportOnboardingState();
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

    return basePrevButton;
  }, [prevOnClick]);

  return (activeStep !== ImportOnboardingPage.Done) ? PrevButtonComponent : null;
}

interface CreateWalletLayoutProps {
  children: React.ReactElement;
}

export function CreateWalletViaImportLayout({
  children,
}: CreateWalletLayoutProps) {
  const { colorMode } = useColorMode();
  const {
    activeStep,
  } = useImportOnboardingState();
  const mnemonic = generateMnemonic();
  const methods = useForm<CreateWalletViaImportGeneralFormValues>({
    defaultValues: {
      confirmPassword: '',
      initialPassword: '',
      mnemonic: mnemonic.split(' '),
      'mnemonic-a': '',
      'mnemonic-b': '',
      'mnemonic-c': '',
      'mnemonic-d': '',
      'mnemonic-e': '',
      'mnemonic-f': '',
      'mnemonic-g': '',
      'mnemonic-h': '',
      'mnemonic-i': '',
      'mnemonic-j': '',
      'mnemonic-k': '',
      'mnemonic-l': '',
      mnemonicString: mnemonic,
      privateKey: '',
    },
  });

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
            activeStep={ImportOnboardingPageEnumDict[activeStep]}
            colorScheme="teal"
            orientation="horizontal"
            responsive={false}
          >
            {createViaImportSteps.map(({ label }, index) => (
              <Step label={(index === activeStep) ? label : label.substring(0, 7)} key={label} />
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
