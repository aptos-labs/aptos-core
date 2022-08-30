// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { SmallCloseIcon, SmallAddIcon, CheckIcon } from '@chakra-ui/icons';
import {
  Avatar, AvatarBadge, Box,
} from '@chakra-ui/react';
import { formatAddress, isAddressValid } from 'core/utils/address';
import React, { useCallback, useMemo } from 'react';
import { GraceHopperBoringAvatar } from './BoringAvatar';

interface TransferAvatarProps {
  doesRecipientAccountExist?: boolean;
  recipient?: string;
}

export default function TransferAvatar({
  doesRecipientAccountExist,
  recipient,
}: TransferAvatarProps) {
  const validRecipientAddress = isAddressValid(recipient) ? formatAddress(recipient) : undefined;

  const getAvatarBadgeColor = useCallback(() => {
    if (!validRecipientAddress) {
      return 'red';
    }
    return 'teal';
  }, [validRecipientAddress]);

  const getAvatarBadgeIcon = useCallback(() => {
    if (!validRecipientAddress) {
      return <SmallCloseIcon color="white" fontSize="xs" />;
    }
    if (!doesRecipientAccountExist) {
      return <SmallAddIcon color="white" fontSize="xs" />;
    }
    return <CheckIcon color="white" fontSize="xs" />;
  }, [doesRecipientAccountExist, validRecipientAddress]);

  const avatarBadge = useMemo(() => {
    const badgeColor = getAvatarBadgeColor();
    const badgeIcon = getAvatarBadgeIcon();
    return (
      <AvatarBadge bg={badgeColor} boxSize="1.25em">
        {badgeIcon}
      </AvatarBadge>
    );
  }, [getAvatarBadgeColor, getAvatarBadgeIcon]);

  return (
    <Box width="48px">
      <Avatar icon={<GraceHopperBoringAvatar type={(doesRecipientAccountExist) ? 'beam' : 'marble'} />}>
        {avatarBadge}
      </Avatar>
    </Box>
  );
}
