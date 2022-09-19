// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo, useState, useRef } from 'react';
import Routes from 'core/routes';
import CreateAccountBody from 'core/components/CreateAccountBody';
import { CreateAccountLayout } from 'core/layouts/AddAccountLayout';
import { useNavigate } from 'react-router-dom';
import { generateMnemonic } from 'core/utils/account';
import { Transition, type TransitionStatus } from 'react-transition-group';
import ConfirmationPopup from 'core/components/ConfirmationPopup';
import useCreateAccount from 'core/hooks/useCreateAccount';
import { BsFillShieldFill } from '@react-icons/all-files/bs/BsFillShieldFill';
import { Box } from '@chakra-ui/react';

const transitionDuration = 200;

function Logo() {
  return (
    <Box bgColor="rgba(0, 191, 165, 0.1)" borderRadius={100} width="75px" height="75px" display="flex" justifyContent="center" alignItems="center">
      <BsFillShieldFill size={36} color="teal" />
    </Box>
  );
}

function CreateAccount() {
  const navigate = useNavigate();
  const { createAccount } = useCreateAccount({});
  const newMnemonic = useMemo(() => generateMnemonic(), []);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const ref = useRef();
  const [
    showSecretRecoveryPhrasePopup,
    setShowSecretRecoveryPhrasePopup,
  ] = useState<boolean>(false);

  const onSubmit = async () => {
    setShowSecretRecoveryPhrasePopup(true);
  };

  const initAccount = async () => {
    setIsLoading(true);
    await createAccount();
    setIsLoading(false);
  };

  return (
    <CreateAccountLayout
      headerValue="Create account"
      backPage={Routes.addAccount.path}
      defaultValues={{
        mnemonic: newMnemonic.split(' '),
        mnemonicString: newMnemonic,
      }}
      onSubmit={onSubmit}
    >
      <CreateAccountBody
        isLoading={isLoading}
        mnemonic={newMnemonic}
      />
      <Transition in={showSecretRecoveryPhrasePopup} timeout={transitionDuration} nodeRef={ref}>
        {(state: TransitionStatus) => (
          <ConfirmationPopup
            bodyWidth="260px"
            logo={<Logo />}
            open={showSecretRecoveryPhrasePopup}
            duration={transitionDuration}
            title="Keep your phrase safe!"
            body="If you lose it you&apos;ll have no way of accessing your assets."
            primaryBttnLabel="Done"
            primaryBttnOnClick={async () => {
              await initAccount();
              navigate(Routes.welcome.path);
            }}
            secondaryBttnLabel="Show phrase again"
            secondaryBttnOnClick={() => {
              setShowSecretRecoveryPhrasePopup(false);
            }}
            state={state}
            isLoading={isLoading}
          />
        )}
      </Transition>
    </CreateAccountLayout>
  );
}

export default CreateAccount;
