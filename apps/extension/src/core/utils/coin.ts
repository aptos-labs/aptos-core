// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import numeral from 'numeral';

export const APTOS_UNIT = 'APT' as const;
export const OCTA_UNIT = 'OCTA' as const;
export const PLURAL_OCTA_UNIT = `${OCTA_UNIT}S` as const;
export const OCTA_NUMBER = 8 as const;
export const OCTA_NEGATIVE_EXPONENT = 1e-8 as const;
export const OCTA_POSITIVE_EXPONENT = 1e8 as const;

interface GenerateUnitStringParams {
  isLowercase: boolean;
  unitType: string;
  usePlural: boolean;
  value?: number;
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
  if (usePlural && value !== 1 && value !== 0) {
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
const generateNumeralFormat = (decimals: 0 | 2 | 4 | 8) => {
  switch (decimals) {
    case 0:
      return '0,0';
    case 2:
      return '0,0.[00]';
    case 8:
      return '0,0.[00000000]';
    default:
      return '0,0.[0000]';
  }
};

interface NumeralTransformerParams {
  format: ReturnType<typeof generateNumeralFormat>;
  multiplier: number;
  value: number;
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
  const signMultiplier = (value < 0) ? -1 : 1;
  const valByMultiplier = signMultiplier * value * multiplier;

  // the if condition should always be positive
  if (value > 0 && valByMultiplier < 1e-6) {
    return numeral(signMultiplier * valByMultiplier * 100)
      .format(format)
      .replace('0.0', '0.000');
  }
  return numeral(signMultiplier * valByMultiplier).format(format);
};

interface FormatCoinOptions {
  decimals?: 0 | 2 | 4 | 8;
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
export const formatCoin = (value?: number, opts: FormatCoinOptions = {}) => {
  if (opts.isNonNegative && value && value < 0) {
    throw new Error('Value cannot be negative');
  }
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
      value: value ?? 0,
    });
  } else {
    transformedNumeral = numeralTransformer({
      format: numeralFormat,
      multiplier,
      value: value ?? 0,
    });
  }

  // add units
  let units: string | null = null;
  if (includeUnit) {
    units = generateUnitString({
      isLowercase,
      unitType: returnUnitType,
      usePlural,
      value,
    });
  }

  const result = (includeUnit) ? `${transformedNumeral} ${units}` : transformedNumeral;
  return result;
};
