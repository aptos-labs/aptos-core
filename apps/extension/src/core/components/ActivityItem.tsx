// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Circle, HStack, Text, Tooltip, useClipboard, useColorMode, VStack,
} from '@chakra-ui/react';
import { HiDownload } from '@react-icons/all-files/hi/HiDownload';
import { BsArrowUpRight } from '@react-icons/all-files/bs/BsArrowUpRight';
import { UserTransaction } from 'aptos/src/api/data-contracts';
import { ScriptFunctionPayload } from 'aptos/dist/api/data-contracts';

/**
 * Convert a timestamp into a relative time short string. If the time difference
 * is above `threshold`, a short date is returned instead
 * @param ts timestamp in milliseconds
 * @param thresholdInDays
 */
function relativeTime(ts: number, thresholdInDays: number = 7) {
  const secondsInMinute = 60;
  const secondsInHour = secondsInMinute * 60;
  const secondsInDay = secondsInHour * 24;

  const seconds = (Date.now() - ts) / 1000;

  if (seconds < secondsInMinute) {
    return `${Math.round(seconds)}s`;
  }
  if (seconds < secondsInHour) {
    return `${Math.round(seconds / secondsInMinute)}m`;
  }
  if (seconds < secondsInDay) {
    return `${Math.round(seconds / secondsInHour)}h`;
  }
  if (seconds < secondsInDay * thresholdInDays) {
    return `${Math.round(seconds / secondsInDay)}d`;
  }

  // Return short date
  return new Date(ts).toLocaleDateString('en-us', { day: 'numeric', month: 'short' });
}

interface ActivityItemProps {
  isSent: boolean,
  transaction: UserTransaction,
}

const secondaryGridHoverBgColor = {
  dark: 'gray.600',
  light: 'gray.200',
};

const secondaryGridBgColor = {
  dark: 'gray.700',
  light: 'gray.100',
};

const secondaryTextColor = {
  dark: 'white',
  light: 'black',
};

const timestampColor = {
  dark: 'gray.500',
  light: 'gray.500',
};

export function ActivityItem({ isSent, transaction }: ActivityItemProps) {
  const { colorMode } = useColorMode();

  const typedPayload = transaction.payload as ScriptFunctionPayload;
  const [recipient, amount]: string[] = typedPayload.arguments;

  const otherAddress = isSent ? recipient : transaction.sender;
  const collapsedAddress = `${otherAddress.slice(0, 5)}..${otherAddress.slice(-4)}`;

  const coinName = typedPayload.type_arguments[0].split('::').pop();

  const timestampMs = Number(transaction.timestamp) / 1000;

  const formattedDate = new Date(timestampMs).toLocaleDateString('en-us', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });

  const formattedTime = new Date(timestampMs).toLocaleTimeString('en-us', {
    hour: 'numeric',
    minute: 'numeric',
  });

  const {
    hasCopied: hasCopiedAddress,
    onCopy: copyAddress,
  } = useClipboard(otherAddress);

  return (
    <HStack
      w="100%"
      spacing={4}
      padding={3}
      paddingLeft={4}
      paddingRight={4}
      color={secondaryTextColor[colorMode]}
      bgColor={secondaryGridBgColor[colorMode]}
      borderRadius=".5rem"
      _hover={{
        bgColor: secondaryGridHoverBgColor[colorMode],
      }}
    >
      <Circle size={8} border="1px" borderColor="blue.400" color="blue.400">
        { isSent ? <BsArrowUpRight /> : <HiDownload /> }
      </Circle>
      <VStack flexGrow={1} alignItems="start" spacing={0.5}>
        <HStack w="100%" fontSize="sm">
          <Text flexGrow={1}>
            { `${isSent ? 'To' : 'From'} ` }
            <Tooltip label={hasCopiedAddress ? 'Copied!' : 'Copy address'} closeDelay={500}>
              <Text cursor="pointer" as="span" onClick={copyAddress}>
                { collapsedAddress }
              </Text>
            </Tooltip>
          </Text>
          <Text color={isSent ? 'red.500' : 'green.500'} fontWeight={500}>
            { `${isSent ? '-' : '+'}${amount} ${coinName}` }
          </Text>
        </HStack>
        <Text color={timestampColor[colorMode]} fontSize="xs">
          <Tooltip label={`${formattedDate} at ${formattedTime}`}>
            { relativeTime(timestampMs) }
          </Tooltip>
        </Text>
      </VStack>
    </HStack>
  );
}

export default ActivityItem;
