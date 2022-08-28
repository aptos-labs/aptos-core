// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack, Input, Text, useColorMode,
} from '@chakra-ui/react';
import { secondaryTextColor } from 'core/colors';
import numeral from 'numeral';
import React, { useMemo } from 'react';
import { useFormContext } from 'react-hook-form';
import MaskedInput from 'react-text-mask';
import { createNumberMask } from 'text-mask-addons';
import { keyframes } from '@emotion/react';
import type { CoinTransferFormData } from './TransferDrawer';

const bounce = keyframes`
  from, 20%, 53%, 80%, to {
    transform: translate3d(0,0,0);
  }

  30% {
    transform: translate3d(30px, 0, 0);
  }

  40%, 43% {
    transform: translate3d(-15px, 0, 0);
  }

  60% {
    transform: translate3d(4px, 0, 0);
  }

  70% {
    transform: translate3d(-15px, 0, 0);
  }

  90% {
    transform: translate3d(-4px, 0, 0);
  }
`;

const defaultMaskOptions = {
  allowDecimal: true,
  allowLeadingZeroes: false,
  // limit length of integer numbers
  allowNegative: false,
  decimalLimit: 8,
  decimalSymbol: '.',
  includeThousandsSeparator: true,
  prefix: '',
  // how many digits allowed after the decimal
  suffix: ' APT',
  thousandsSeparatorSymbol: ',',
};

function getAmountInputFontSize(amount?: number) {
  if (!amount || amount < 1e4) {
    return 64;
  }
  if (amount < 1e7) {
    return 48;
  }
  return 36;
}

const currencyMask = createNumberMask(defaultMaskOptions);

interface TransferInputProps {
  coinBalance?: number;
  doesRecipientAccountExist?: boolean,
  estimatedGasFee?: number;
  shouldBalanceShake?: boolean;
}

export default function TransferInput({
  coinBalance,
  estimatedGasFee,
  shouldBalanceShake,
}: TransferInputProps) {
  const {
    register,
    watch,
  } = useFormContext<CoinTransferFormData>();
  const { colorMode } = useColorMode();
  const amount = watch('amount');
  const numberAmount = numeral(amount).value() || undefined;
  const coinBalanceString = numeral(coinBalance).format('0,0');
  const amountInputFontSize = useMemo(() => getAmountInputFontSize(numberAmount), [numberAmount]);

  const {
    onChange: amountOnChange,
    ref: amountRef,
  } = register('amount');

  const inputOnChange = (
    e: React.ChangeEvent<HTMLInputElement> | undefined,
    maskedInputOnChangeCallback: (event: React.ChangeEvent<HTMLElement>) => void,
  ): void => {
    amountOnChange(e!);
    maskedInputOnChangeCallback(e!);
  };

  return (
    <VStack spacing={0} position="relative">
      <MaskedInput
        mask={currencyMask}
        render={(ref, props) => (
          <Input
            {...props}
            autoComplete="off"
            textAlign="center"
            variant="filled"
            placeholder="0"
            py={24}
            pb={32}
            fontSize={amountInputFontSize}
            borderRadius="0px"
            backgroundColor="transparent"
            _focusVisible={{
              outline: 'none',
            }}
            {...register('amount', { valueAsNumber: false })}
            // eslint-disable-next-line react/prop-types
            onChange={(e) => inputOnChange(e, props.onChange)}
            ref={(e) => {
              ref(e!);
              amountRef(e);
            }}
          />
        )}
      />
      <Text
        fontSize="sm"
        color={secondaryTextColor[colorMode]}
        position="absolute"
        bottom={16}
        animation={(shouldBalanceShake) ? `${bounce} 1s ease infinite` : undefined}
      >
        Balance:
        {' '}
        {`${coinBalanceString} APT`}
        ,
        fees:
        {' '}
        {`${estimatedGasFee || 0} APT`}
      </Text>
    </VStack>
  );
}
