// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import Avatar from 'boring-avatars';
import React from 'react';

interface AvatarProps {
  address: string,
  size: number;
}

export default function AvatarImage({ address, size }: AvatarProps) {
  return (
    <Avatar
      size={size}
      name={address}
      variant="bauhaus"
    />
  );
}
