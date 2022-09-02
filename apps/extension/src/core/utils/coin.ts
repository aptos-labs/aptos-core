// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import numeral from 'numeral';

export const APTOS_UNIT = 'APT' as const;
export const OCTA_UNIT = 'OCTA' as const;
export const OCTA_NUMBER = 8 as const;
export const OCTA_EXPONENT = 1e-8 as const;

export interface OctaToAptParams {
  octas?: number;
  octasCanBeUndefined?: boolean;
}

const octaToApt = ({
  octas,
  octasCanBeUndefined = true,
}: OctaToAptParams) => {
  if (octasCanBeUndefined) {
    return (octas || 0) * OCTA_EXPONENT;
  }
  if (octas) {
    return (octas) * OCTA_EXPONENT;
  }
  throw new Error('Octas provided were undefined');
};
export const aptWithDecimals = (apt?: number) => numeral(apt).format('0,0.0000');

export interface OctaToAptWithDecimalsParams {
  decimals?: 0 | 2 | 4 | 8;
  octas?: number;
  withUnit?: boolean;
}

export const octaToAptWithDecimals = ({
  decimals = 4,
  octas,
  withUnit = true,
}: OctaToAptWithDecimalsParams) => {
  let format: string = '';
  switch (decimals) {
    case 0:
      format = '0,0';
      break;
    case 2:
      format = '0,0.00';
      break;
    case 8:
      format = '0,0.00000000';
      break;
    default:
      format = '0,0.0000';
      break;
  }
  format = (withUnit) ? (`${format} ${APTOS_UNIT}`) : format;
  console.log(octaToApt({ octas }));
  return numeral(octaToApt({ octas })).format(format);
};
