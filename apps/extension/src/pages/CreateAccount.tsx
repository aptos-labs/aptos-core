// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo, useState, useRef } from 'react';
import Routes from 'core/routes';
import CreateAccountBody from 'core/components/CreateAccountBody';
import { CreateAccountLayout } from 'core/layouts/AddAccountLayout';
import { useNavigate } from 'react-router-dom';
import { AptosAccount } from 'aptos';
import { generateMnemonic, generateMnemonicObject, keysFromAptosAccount } from 'core/utils/account';
import { Transition, type TransitionStatus } from 'react-transition-group';
import SecretPhraseConfirmationPopup from 'core/components/SecretPhraseConfirmationPopup';
import { useUnlockedAccounts } from 'core/hooks/useAccounts';
import useFundAccount from 'core/mutations/faucet';
import { createAccountErrorToast, createAccountToast } from 'core/components/Toast';
import { useAnalytics } from 'core/hooks/useAnalytics';
import { accountEvents } from 'core/utils/analytics/events';

const transitionDuration = 200;

function CreateAccount() {
  const navigate = useNavigate();
  const { addAccount } = useUnlockedAccounts();
  const { fundAccount } = useFundAccount();
  const newMnemonic = useMemo(() => generateMnemonic(), []);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const { trackEvent } = useAnalytics();
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

    try {
      const { mnemonic, seed } = await generateMnemonicObject(newMnemonic);
      const aptosAccount = new AptosAccount(seed);

      const newAccount = {
        mnemonic,
        ...keysFromAptosAccount(aptosAccount),
      };
      await addAccount(newAccount);

      if (fundAccount) {
        await fundAccount({ address: newAccount.address, amount: 0 });
      }

      trackEvent({
        eventType: accountEvents.CREATE_ACCOUNT,
      });

      createAccountToast();
    } catch (err) {
      trackEvent({
        eventType: accountEvents.ERROR_CREATE_ACCOUNT,
        params: {
          error: String(err),
        },
      });
      createAccountErrorToast();
      // eslint-disable-next-line no-console
      console.error(err);
    }
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
          <SecretPhraseConfirmationPopup
            open={showSecretRecoveryPhrasePopup}
            duration={transitionDuration}
            state={state}
            isLoading={isLoading}
            goPrev={() => {
              setShowSecretRecoveryPhrasePopup(false);
            }}
            goNext={async () => {
              await initAccount();
              navigate(Routes.wallet.path);
            }}
          />
        )}
      </Transition>
    </CreateAccountLayout>
  );
}

export default CreateAccount;
