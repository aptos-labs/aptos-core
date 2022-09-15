// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Button,
} from '@chakra-ui/react';
import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import { useActiveAccount } from 'core/hooks/useAccounts';
import useFundAccount from 'core/mutations/faucet';
import { defaultFundAmount } from 'core/constants';

export default function Faucet() {
  const { activeAccountAddress } = useActiveAccount();
  const { fundAccount, isFunding } = useFundAccount();

  const onClick = async () => {
    if (!fundAccount) {
      return;
    }

    await fundAccount({ address: activeAccountAddress, amount: defaultFundAmount });
  };

  return (
    <Button
      isLoading={isFunding}
      leftIcon={<FontAwesomeIcon icon={faFaucet} />}
      onClick={onClick}
      isDisabled={isFunding}
      backgroundColor="whiteAlpha.200"
      _hover={{
        backgroundColor: 'whiteAlpha.300',
      }}
      _active={{
        backgroundColor: 'whiteAlpha.400',
      }}
      color="white"
      variant="solid"
    >
      Faucet
    </Button>
  );
}
