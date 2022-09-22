// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Circle,
  HStack,
  Text,
  Tooltip,
  useColorMode,
  useInterval,
  VStack,
} from '@chakra-ui/react';
import React, { useState } from 'react';
import {
  secondaryGridBgColor,
  secondaryGridHoverBgColor,
  timestampColor,
} from 'core/colors';
import ChakraLink from 'core/components/ChakraLink';

/**
 * Convert a timestamp into a relative time short string. If the time difference
 * is above `threshold`, a short date is returned instead
 * @param ts timestamp in milliseconds
 * @param thresholdInDays
 */
function getRelativeTime(ts: number, thresholdInDays: number = 7) {
  const secondsInMinute = 60;
  const secondsInHour = secondsInMinute * 60;
  const secondsInDay = secondsInHour * 24;

  const seconds = (Date.now() - ts) / 1000;

  if (seconds < secondsInMinute) {
    return 'Moments ago';
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

function getAbsoluteDateTime(timestampMs: number) {
  const formattedDate = new Date(timestampMs).toLocaleDateString('en-us', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
  const formattedTime = new Date(timestampMs).toLocaleTimeString('en-us', {
    hour: 'numeric',
    minute: 'numeric',
  });
  return `${formattedDate} at ${formattedTime}`;
}

function useRelativeTime(ts: number, updateIntervalMs = 5000) {
  const [value, setValue] = useState<string>(getRelativeTime(ts));
  useInterval(() => {
    setValue(getRelativeTime(ts));
  }, updateIntervalMs);
  return value;
}

export interface ActivityItemProps {
  amount: string,
  amountColor: string,
  icon: JSX.Element,
  text: string,
  timestamp: number,
  txnVersion: number
}

export function ActivityListItem({
  amount,
  amountColor,
  icon,
  text,
  timestamp,
  txnVersion,
}: ActivityItemProps) {
  const { colorMode } = useColorMode();
  const absDateTime = getAbsoluteDateTime(timestamp);
  const relTime = useRelativeTime(timestamp);

  return (
    <ChakraLink to={`/transactions/${txnVersion}`} w="100%">
      <HStack
        spacing={4}
        py={3}
        px={4}
        cursor="pointer"
        bgColor={secondaryGridBgColor[colorMode]}
        borderRadius=".5rem"
        _hover={{
          bgColor: secondaryGridHoverBgColor[colorMode],
        }}
      >
        <Circle size={8} border="1px" borderColor="blue.400" color="blue.400">
          { icon }
        </Circle>
        <VStack flexGrow={1} alignItems="start" spacing={0.5}>
          <HStack w="100%" fontSize="sm">
            <Text flexGrow={1}>
              { text }
            </Text>
            <Text
              maxWidth="45%"
              color={amountColor}
              fontWeight={500}
              whiteSpace="nowrap"
              overflow="hidden"
              textOverflow="ellipsis"
            >
              { amount }
            </Text>
          </HStack>
          <Text color={timestampColor[colorMode]} fontSize="xs">
            <Tooltip label={absDateTime}>{ relTime }</Tooltip>
          </Text>
        </VStack>
      </HStack>
    </ChakraLink>
  );
}

export default ActivityListItem;
