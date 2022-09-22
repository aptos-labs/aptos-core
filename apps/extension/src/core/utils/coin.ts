// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import numeral from 'numeral';
import { CoinInfoData } from 'shared/types';

export const APTOS_UNIT = 'APT' as const;
export const OCTA_UNIT = 'OCTA' as const;
export const PLURAL_OCTA_UNIT = `${OCTA_UNIT}S` as const;
export const OCTA_NUMBER: number = 8 as const;
export const OCTA_NEGATIVE_EXPONENT = 10 ** (-OCTA_NUMBER);
export const OCTA_POSITIVE_EXPONENT = 10 ** OCTA_NUMBER;

interface GenerateUnitStringParams {
  isLowercase: boolean;
  unitType: string;
  usePlural: boolean;
  value?: bigint;
}

/**
 * Generates the unit string for a coin (ie. APT | OCTA)
 * Can be configured to add an "S" if usePluralUnit is true
 */
const generateUnitString = ({
  isLowercase,
  unitType = APTOS_UNIT,
  usePlural,
  value,
}: GenerateUnitStringParams) => {
  let result: GenerateUnitStringParams['unitType'] | typeof PLURAL_OCTA_UNIT = unitType;
  if (usePlural && value !== 1n && value !== 0n) {
    switch (unitType) {
      case 'APT':
        result = unitType;
        break;
      case 'OCTA':
        result = `${unitType}S`;
        break;
      default:
        result = unitType;
        break;
    }
  }
  return (isLowercase) ? result.toLowerCase() : result;
};

/**
 * Generates numeral format based on number of decimals
 * brackets (ie. [0000]) indicates format always rounds to the nearest number
 * with that format
 */
const generateNumeralFormat = (decimals: number) => {
  switch (decimals) {
    case 0:
      return '0,0';
    case 2:
      return '0,0.[00]';
    case 4:
      return '0,0.[0000]';
    case 8:
      return '0,0.[00000000]';
    default: {
      let decimalsString = '';
      for (let x = 0; x < decimals; x += 1) {
        decimalsString += '0';
      }
      return `0,0.[${decimalsString}]`;
    }
  }
};

interface NumeralTransformerParams {
  format: ReturnType<typeof generateNumeralFormat>;
  multiplier: number;
  value: bigint;
}

function zeroPad(number: bigint, decimals: number) {
  const zero = decimals - number.toString().length + 1;
  return Array(+(zero > 0 && zero)).join('0') + number;
}

/**
 * Unfortunately numeral has issues working with numbers past a certain size
 * numeralTransformer is an automatic workaround to numeral NaN issues
 * on small numbers -> https://bit.ly/3Ry6S63
 */
const numeralTransformer = ({
  format,
  multiplier,
  value,
}: NumeralTransformerParams) => {
  const inverseMultiplier = multiplier ** -1;
  const integral = value / BigInt(inverseMultiplier);
  const fractional = value % BigInt(inverseMultiplier);

  // If number is < 1e-6, we need to workaround https://bit.ly/3Ry6S63
  if (value > 0 && integral === 0n && fractional < 1e2) {
    const newFractional = fractional * 100n;
    const paddedFractional = zeroPad(newFractional, inverseMultiplier.toString().length - 1);
    return numeral(`${integral}.${paddedFractional}`)
      .format(format)
      .replace('0.0', '0.000');
  }

  const paddedFractional = zeroPad(fractional, inverseMultiplier.toString().length - 1);
  return numeral(`${integral}.${paddedFractional}`).format(format);
};

interface FormatCoinOptions {
  decimals?: number;
  includeUnit?: boolean;
  isLowercase?: boolean;
  isNonNegative?: boolean;
  multiplier?: number;
  paramUnitType?: typeof APTOS_UNIT | typeof OCTA_UNIT;
  returnUnitType?: typeof APTOS_UNIT | typeof OCTA_UNIT;
  usePlural?: boolean;
}

export const aptToOcta = (octa: number) => octa * OCTA_POSITIVE_EXPONENT;

/**
 * Used for formatting all Aptos coins in different units, like OCTA (10^-8 APT)
 * can be easily extended in the future to include custom coins
 * @param {Number} value The value that a coin has
 * @param {FormatCoinParams} opts Specify custom properties for formatting the coin
 */
export const formatCoin = (value?: bigint | number, opts: FormatCoinOptions = {}) => {
  if (opts.isNonNegative && value && value < 0) {
    throw new Error('Value cannot be negative');
  }
  const coinValue = (typeof value === 'bigint') ? value : BigInt(value ?? 0);
  const {
    decimals = 4,
    includeUnit = true,
    multiplier = OCTA_NEGATIVE_EXPONENT,
    returnUnitType = 'APT',
    paramUnitType = 'OCTA',
    usePlural = true,
    isLowercase = false,
  } = opts;
  const numeralFormat = generateNumeralFormat(decimals);

  // Format the numeral
  let transformedNumeral: string;
  if (returnUnitType === paramUnitType) {
    transformedNumeral = numeralTransformer({
      format: numeralFormat,
      multiplier: 1,
      value: coinValue,
    });
  } else {
    transformedNumeral = numeralTransformer({
      format: numeralFormat,
      multiplier,
      value: coinValue,
    });
  }

  // add units
  let units: string | null = null;
  if (includeUnit) {
    units = generateUnitString({
      isLowercase,
      unitType: returnUnitType,
      usePlural,
      value: coinValue,
    });
  }

  const result = (includeUnit) ? `${transformedNumeral} ${units}` : transformedNumeral;
  return result;
};

interface FormatAmountOptions {
  decimals?: number,
  prefix?: boolean,
}

export function formatAmount(
  amount: number | bigint,
  coinInfo: CoinInfoData | undefined,
  options?: FormatAmountOptions,
) {
  const { decimals, prefix } = {
    decimals: 8,
    prefix: true,
    ...options,
  };

  const amountSign = amount > 0 ? '+' : '-';
  const amountAbs = amount > 0 ? amount : -amount;
  const multiplier = coinInfo?.decimals ? 10 ** (-coinInfo.decimals) : 1;
  const amountPrefix = prefix ? amountSign : '';
  const amountSuffix = coinInfo?.symbol ? ` ${coinInfo.symbol}` : '';
  const formattedAmount = formatCoin(amountAbs, {
    decimals,
    includeUnit: false,
    multiplier,
  });

  return `${amountPrefix}${formattedAmount}${amountSuffix}`;
}
