// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback } from 'react';
import Routes from 'core/routes';
import { ImportAccountPrivateKeyLayout, PrivateKeyFormValues } from 'core/layouts/AddAccountLayout';
import ImportAccountPrivateKeyBody from 'core/components/ImportAccountPrivateKeyBody';
import { AptosAccount } from 'aptos';
import { useNavigate } from 'react-router-dom';
import { importAccountErrorToast, importAccountToast } from 'core/components/Toast';
import { useUnlockedAccounts } from 'core/hooks/useAccounts';
import { keysFromAptosAccount } from 'core/utils/account';
import { useAnalytics } from 'core/hooks/useAnalytics';
import { importAccountEvents } from 'core/utils/analytics/events';

export default function ImportAccountPrivateKey() {
  const navigate = useNavigate();
  const { addAccount } = useUnlockedAccounts();
  const { trackEvent } = useAnalytics();

  const onSubmit = useCallback(async (
    data: PrivateKeyFormValues,
    event?: React.BaseSyntheticEvent,
  ) => {
    const { privateKey } = data;
    event?.preventDefault();
    try {
      const nonHexKey = (privateKey.startsWith('0x')) ? privateKey.substring(2) : privateKey;
      const encodedKey = Uint8Array.from(Buffer.from(nonHexKey, 'hex'));
      const aptosAccount = new AptosAccount(encodedKey);
      // TODO: prompt user for confirmation if account is not on chain

      await addAccount(keysFromAptosAccount(aptosAccount));

      importAccountToast();

      trackEvent({ eventType: importAccountEvents.IMPORT_PK_ACCOUNT });
      navigate(Routes.wallet.path);
    } catch (err) {
      trackEvent({
        eventType: importAccountEvents.ERROR_IMPORT_PK_ACCOUNT,
        params: {
          error: String(err),
        },
      });
      importAccountErrorToast();
    }
  }, [addAccount, navigate, trackEvent]);

  return (
    <ImportAccountPrivateKeyLayout
      headerValue="Import private key"
      backPage={Routes.addAccount.path}
      defaultValues={{
        privateKey: '',
      }}
      onSubmit={onSubmit}
    >
      <ImportAccountPrivateKeyBody hasSubmit />
    </ImportAccountPrivateKeyLayout>
  );
}
