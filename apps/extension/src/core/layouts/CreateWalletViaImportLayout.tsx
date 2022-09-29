// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box, Button, Flex, Grid, Tooltip, useColorMode, HStack, IconButton,
} from '@chakra-ui/react';
import { secondaryBgColor, secondaryBackButtonBgColor } from 'core/colors';
import { useImportOnboardingState } from 'core/hooks/useImportOnboardingState';
import React, { useCallback, useMemo, useState } from 'react';
import { FormProvider, useForm, useFormContext } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';
import { ArrowBackIcon } from '@chakra-ui/icons';
import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { passwordOptions } from 'core/components/CreatePasswordBody';
import { generateMnemonic, generateMnemonicObject } from 'core/utils/account';
import { AptosAccount } from 'aptos';
import {
  importAccountErrorToast, importAccountToast, networkDoesNotExistToast,
} from 'core/components/Toast';
import { useAccounts } from 'core/hooks/useAccounts';
import { useNetworks } from 'core/hooks/useNetworks';
import { passwordStrength, mnemonicValues } from 'core/constants';
import Step from 'core/components/Step';
import { lookUpAndInitAccounts } from 'core/utils/rotateKey';
import { MNEMONIC } from 'core/enums';
import { MnemonicFormValues } from './AddAccountLayout';

zxcvbnOptions.setOptions(passwordOptions);

// These enum are like this so that it will work with the Step component
// to show correct active/complete state
export enum ImportOnboardingPage {
  ImportType = 0,
  ImportMnemonicOrPrivateKey = 1,
  ImportMnemonic = 0.25,
  ImportPrivateKey = 0.75,
  CreatePassword = 2,
  Done = 3,
}

const createViaImportSteps = [
  {
    label: 'Import type',
    name: ImportOnboardingPage.ImportType,
    substeps: [],
  },
  {
    label: 'Enter mnemonic/private key',
    name: ImportOnboardingPage.ImportMnemonicOrPrivateKey,
    substeps: [
      {
        label: 'Enter mnemonic',
        name: ImportOnboardingPage.ImportMnemonic,
      },
      {
        label: 'Enter private key',
        name: ImportOnboardingPage.ImportPrivateKey,
      },
    ],
  },
  {
    label: 'Create password',
    name: ImportOnboardingPage.CreatePassword,
    substeps: [],
  },
];

export interface CreateWalletViaImportFormValues {
  confirmPassword: string;
  importType: 'privateKey' | 'mnemonic';
  initialPassword: string;
  mnemonic: string[];
  mnemonicString: string;
  privateKey: string;
  secretRecoveryPhrase: boolean;
  showPrivateKey: boolean;
  termsOfService: boolean;
}

export type CreateWalletViaImportGeneralFormValues =
CreateWalletViaImportFormValues & MnemonicFormValues;

const buttonBorderColor = {
  dark: 'gray.700',
  light: 'gray.200',
};

function NextButton() {
  const { setValue, watch } = useFormContext<CreateWalletViaImportGeneralFormValues>();
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const { activeNetwork, aptosClient } = useNetworks();

  const {
    initAccounts,
  } = useAccounts();

  const {
    activeStep, nextStep, setActiveStep,
  } = useImportOnboardingState();

  const termsOfService = watch('termsOfService');
  const initialPassword = watch('initialPassword');
  const confirmPassword = watch('confirmPassword');
  const privateKey = watch('privateKey');
  const importType = watch('importType');
  const allFields = watch();

  const mnemonicArray = useMemo(() => mnemonicValues.map((m) => allFields[m].trim()), [allFields]);
  const passwordResult = zxcvbn(initialPassword);
  const passwordScore = passwordResult.score;

  const initAccountWithMnemonic = useCallback(async () => {
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

      // initialize password and wallet
      await lookUpAndInitAccounts({
        aptosAccount, aptosClient, confirmPassword, initAccounts, mnemonic,
      });

      setIsLoading(false);
      importAccountToast();
    } catch (err) {
      setIsLoading(false);
      importAccountErrorToast();
    }
    nextStep();
  }, [
    activeNetwork,
    aptosClient,
    confirmPassword,
    initAccounts,
    mnemonicArray,
    nextStep,
  ]);

  const intiAccountWithPrivateKey = useCallback(async () => {
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

      // initialize password and wallet
      await lookUpAndInitAccounts({
        aptosAccount, aptosClient, confirmPassword, initAccounts,
      });

      setIsLoading(false);
      importAccountToast();
    } catch (err) {
      setIsLoading(false);
      importAccountErrorToast();
    }
    nextStep();
  }, [
    aptosClient,
    activeNetwork,
    confirmPassword,
    nextStep,
    initAccounts,
    privateKey,
  ]);

  const nextOnClick = useCallback(async () => {
    switch (activeStep) {
      case ImportOnboardingPage.ImportType: {
        setActiveStep(importType === 'mnemonic' ? ImportOnboardingPage.ImportMnemonic : ImportOnboardingPage.ImportPrivateKey);
        return;
      }
      case ImportOnboardingPage.ImportPrivateKey:
      case ImportOnboardingPage.ImportMnemonic: {
        nextStep();
        return;
      }
      case ImportOnboardingPage.CreatePassword:
        if (importType === 'mnemonic') {
          await initAccountWithMnemonic();
        } else if (importType === 'privateKey') {
          await intiAccountWithPrivateKey();
        }

        // clear out privateKey and mnemonicValues after init account
        // for security purposes
        setValue('privateKey', '');
        mnemonicValues.forEach((v) => {
          setValue(v, '');
        });

        nextStep();
        return;
      case ImportOnboardingPage.Done:
        return;
      default:
        throw new Error('Undefined next step');
    }
  }, [
    setValue,
    activeStep,
    importType,
    nextStep,
    setActiveStep,
    initAccountWithMnemonic,
    intiAccountWithPrivateKey,
  ]);

  const buttonText = useMemo(() => {
    if (activeStep === ImportOnboardingPage.ImportPrivateKey) {
      return 'Import';
    }

    return 'Continue';
  }, [activeStep]);

  const NextButtonComponent = useMemo(() => {
    const baseNextButton = (
      <Button width="100%" height="48px" isLoading={isLoading} onClick={nextOnClick} colorScheme="salmon">
        {buttonText}
      </Button>
    );

    const disabledNextButton = (
      <Button width="100%" height="48px" isLoading={isLoading} isDisabled onClick={nextOnClick} colorScheme="salmon">
        {buttonText}
      </Button>
    );

    switch (activeStep) {
      case ImportOnboardingPage.CreatePassword: {
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
      case ImportOnboardingPage.ImportType: {
        if (importType) {
          return baseNextButton;
        }
        return (
          <Tooltip label="You must choose from the options above">
            {disabledNextButton}
          </Tooltip>
        );
      }
      case ImportOnboardingPage.ImportMnemonic: {
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
      case ImportOnboardingPage.ImportPrivateKey: {
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
    buttonText,
    privateKey.length,
  ]);

  return NextButtonComponent;
}

interface CreateWalletLayoutProps {
  children: React.ReactElement;
}

function CreateWalletViaImportLayoutFC({
  children,
  prevOnClick,
}: CreateWalletLayoutProps & { prevOnClick: () => void }) {
  const { colorMode } = useColorMode();
  const { setValue } = useFormContext<CreateWalletViaImportGeneralFormValues>();
  const {
    activeStep,
  } = useImportOnboardingState();

  const handleClickPrev = () => {
    // clear out private key and mnemonic valuesfor security purposes
    setValue('privateKey', '');
    mnemonicValues.forEach((v) => {
      setValue(v, '');
    });

    prevOnClick();
  };

  return (
    <Grid
      height="100%"
      width="100%"
      maxW="100%"
      templateRows="60px 1fr 84px"
      bgColor={secondaryBgColor[colorMode]}
    >
      <HStack width="100%" px={4}>
        <IconButton
          position="absolute"
          size="md"
          aria-label="back"
          colorScheme="teal"
          icon={<ArrowBackIcon fontSize={20} />}
          variant="filled"
          onClick={handleClickPrev}
          bgColor={secondaryBackButtonBgColor[colorMode]}
          borderRadius="1rem"
        />
        <Flex justifyContent="center" width="100%">
          <HStack spacing="0" justify="space-evenly" width="40%">
            {createViaImportSteps.map(({ label, name, substeps }, id) => (
              (
                <Step
                  key={label}
                  cursor="pointer"
                  isActive={activeStep === name
                      || substeps?.findIndex((s) => s.name === activeStep) !== -1}
                  isCompleted={activeStep > id}
                  isLastStep={id === createViaImportSteps.length - 1}
                />
              )
            ))}
          </HStack>
        </Flex>
      </HStack>
      <Box px={4} height="100%" width="100%" maxH="100%" overflowY="auto">
        <form style={{ height: '100%' }}>
          {children}
        </form>
      </Box>
      <Flex width="100%" px={4} pt={4} borderTop="1px" borderColor={buttonBorderColor[colorMode]}>
        <NextButton />
      </Flex>
    </Grid>
  );
}

export function CreateWalletViaImportLayout(props: CreateWalletLayoutProps) {
  const mnemonic = generateMnemonic();
  const mnemonicHash = mnemonicValues.reduce((acc, v: MNEMONIC) => ({
    ...acc,
    [v]: '',
  }), {});

  const methods = useForm<CreateWalletViaImportGeneralFormValues>({
    defaultValues: {
      confirmPassword: '',
      initialPassword: '',
      mnemonic: mnemonic.split(' '),
      mnemonicString: mnemonic,
      privateKey: '',
      showPrivateKey: false,
      ...mnemonicHash,
    },
  });

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

  return (
    <FormProvider {...methods}>
      <CreateWalletViaImportLayoutFC {...props} prevOnClick={prevOnClick} />
    </FormProvider>
  );
}
