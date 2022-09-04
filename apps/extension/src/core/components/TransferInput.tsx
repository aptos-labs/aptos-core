// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  VStack, Input, Text, useColorMode, Tooltip,
} from '@chakra-ui/react';
import { secondaryTextColor } from 'core/colors';
import React, { useMemo } from 'react';
import MaskedInput from 'react-text-mask';
import { createNumberMask } from 'text-mask-addons';
import { keyframes } from '@emotion/react';
import { APTOS_UNIT } from 'core/utils/coin';
import { useTransferFlow } from 'core/hooks/useTransferFlow';

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
  // this would be more than the supply of APT,
  // we can change the mask options once other coins are introduced
  integerLimit: 10,
  prefix: '',
  // how many digits allowed after the decimal
  suffix: ` ${APTOS_UNIT}`,
  thousandsSeparatorSymbol: ',',
};

function getAmountInputFontSize(amount?: number) {
  // TODO: change so that it is determined by string length, not amount
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const amountStringLength = String(amount).length;
  if (!amount || amount < 1e4) {
    return 64;
  }
  if (amount < 1e7) {
    return 48;
  }
  if (amount < 1e10) {
    return 36;
  }
  if (amount < 1e16) {
    return 24;
  }
  return 18;
}

const currencyMask = createNumberMask(defaultMaskOptions);

export default function TransferInput() {
  const {
    amountAptNumber,
    coinBalanceApt,
    estimatedGasFeeApt,
    formMethods: { register },
    shouldBalanceShake,
  } = useTransferFlow();
  const { colorMode } = useColorMode();
  const amountInputFontSize = useMemo(
    () => getAmountInputFontSize(amountAptNumber),
    [amountAptNumber],
  );

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
      <Tooltip label={`Network fee: ${estimatedGasFeeApt}`}>
        <Text
          fontSize="sm"
          color={secondaryTextColor[colorMode]}
          position="absolute"
          bottom={16}
          animation={(shouldBalanceShake) ? `${bounce} 1s ease infinite` : undefined}
        >
          Balance:
          {' '}
          {coinBalanceApt}
          ,
          fees:
          {' '}
          {estimatedGasFeeApt}
        </Text>
      </Tooltip>
    </VStack>
  );
}
