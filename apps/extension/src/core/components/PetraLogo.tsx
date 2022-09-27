// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { customColors } from 'core/colors';
import React from 'react';

export function PetraSalmonLogo() {
  return (
    <svg
      id="Layer_1"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 1000 1000"
      fill={customColors.salmon[500]}
    >
      <path className="cls-1" d="M473.73,933.7h0c-158.66,0-287.28-128.62-287.28-287.28V170.77S473.73,66.3,473.73,66.3V933.7Z" />
      <path className="cls-1" d="M526.27,576.86h0c158.66,0,287.28-128.62,287.28-287.28v-118.81s-287.28-104.47-287.28-104.47v510.56Z" />
    </svg>
  );
}

export function PetraWhiteLogo() {
  return (
    <svg
      id="Layer_1"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 1000 1000"
      fill="white"
    >
      <path className="cls-1" d="M473.73,933.7h0c-158.66,0-287.28-128.62-287.28-287.28V170.77S473.73,66.3,473.73,66.3V933.7Z" />
      <path className="cls-1" d="M526.27,576.86h0c158.66,0,287.28-128.62,287.28-287.28v-118.81s-287.28-104.47-287.28-104.47v510.56Z" />
    </svg>
  );
}

export function PetraBlueLogo() {
  return (
    <svg
      id="Layer_1"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 1000 1000"
      fill={customColors.navy[900]}
    >
      <path className="cls-1" d="M473.73,933.7h0c-158.66,0-287.28-128.62-287.28-287.28V170.77S473.73,66.3,473.73,66.3V933.7Z" />
      <path className="cls-1" d="M526.27,576.86h0c158.66,0,287.28-128.62,287.28-287.28v-118.81s-287.28-104.47-287.28-104.47v510.56Z" />
    </svg>
  );
}

export function PetraLogo() {
  return <PetraSalmonLogo />;
}
