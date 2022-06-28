// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';

interface BoringAvatarProps {
  type?: 'beam' | 'marble'
}

/**
 * @see https://boringavatars.com/
 */
export function GraceHopperBoringAvatar({
  type = 'beam',
}: BoringAvatarProps) {
  const marble = (
    <svg viewBox="0 0 80 80" fill="none" role="img" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
      <title>Grace Hopper</title>
      <mask id="mask__marble" maskUnits="userSpaceOnUse" x="0" y="0" width="80" height="80"><rect width="80" height="80" rx="160" fill="#FFFFFF" /></mask>
      <g mask="url(#mask__marble)">
        <rect width="80" height="80" fill="#efb0a9" />
        <path filter="url(#prefix__filter0_f)" d="M32.414 59.35L50.376 70.5H72.5v-71H33.728L26.5 13.381l19.057 27.08L32.414 59.35z" fill="#bda0a2" transform="translate(0 0) rotate(-88 40 40) scale(1.2)" />
        <path filter="url(#prefix__filter0_f)" d="M22.216 24L0 46.75l14.108 38.129L78 86l-3.081-59.276-22.378 4.005 12.972 20.186-23.35 27.395L22.215 24z" fill="#ffe6db" transform="translate(4 4) rotate(132 40 40) scale(1.2)" style={{ mixBlendMode: 'overlay' }} />
      </g>
      <defs>
        <filter id="prefix__filter0_f" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feGaussianBlur stdDeviation="7" result="effect1_foregroundBlur" />
        </filter>
      </defs>
    </svg>
  );

  const beam = (
    <svg viewBox="0 0 36 36" fill="none" role="img" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
      <title>Grace Hopper</title>
      <mask id="mask__beam" maskUnits="userSpaceOnUse" x="0" y="0" width="36" height="36"><rect width="36" height="36" rx="72" fill="#FFFFFF" /></mask>
      <g mask="url(#mask__beam)">
        <rect width="36" height="36" fill="#d1eaee" />
        <rect x="0" y="0" width="36" height="36" transform="translate(0 8) rotate(44 18 18) scale(1.2)" fill="#efb0a9" rx="36" />
        <g transform="translate(-4 4) rotate(-4 18 18)">
          <path d="M13,21 a1,0.75 0 0,0 10,0" fill="#000000" />
          <rect x="10" y="14" width="1.5" height="2" rx="1" stroke="none" fill="#000000" />
          <rect x="24" y="14" width="1.5" height="2" rx="1" stroke="none" fill="#000000" />
        </g>
      </g>
    </svg>
  );

  switch (type) {
    case 'marble': return marble;
    case 'beam': return beam;
    default: return beam;
  }
}

export default GraceHopperBoringAvatar;
