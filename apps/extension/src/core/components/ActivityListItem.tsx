// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box,
  Circle,
  Heading,
  HStack,
  Text,
  Tooltip,
  VStack,
  useColorMode,
  useInterval,
} from '@chakra-ui/react';
import React, { useState } from 'react';
import ChakraLink from 'core/components/ChakraLink';
import { BiChevronRight } from '@react-icons/all-files/bi/BiChevronRight';
import { customColors } from 'core/colors';
import { transparentize } from 'color2k';

const itemTextColor = { dark: 'navy.200', light: 'navy.700' };
const iconColor = customColors.green['500'];
const iconBgColor = transparentize(iconColor, 0.9);
const itemHoverColor = transparentize(iconColor, 0.95);

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
    <ChakraLink to={`/transactions/${txnVersion}`} w="100%" m={0}>
      <HStack
        h="100%"
        spacing={3}
        py={4}
        px={4}
        cursor="pointer"
        _hover={{
          bgColor: itemHoverColor,
        }}
      >
        <Circle size="37px" bgColor={iconBgColor} color={iconColor}>
          { icon }
        </Circle>
        <VStack flexGrow={1} alignItems="start" spacing="3px">
          <Heading fontSize="sm" color={itemTextColor[colorMode]}>{ text }</Heading>
          <Text color="navy.600" fontSize="xs">
            <Text as="span">Confirmed</Text>
            <Text as="span" color="navy.700" px={0.5}>&bull;</Text>
            <Tooltip label={absDateTime}>{ relTime }</Tooltip>
          </Text>
        </VStack>
        <Heading
          fontSize="sm"
          alignSelf="start"
          color={amountColor}
          whiteSpace="nowrap"
          overflow="hidden"
          textOverflow="ellipsis"
          textAlign="right"
        >
          { amount }
        </Heading>
        <Box color="navy.500">
          <BiChevronRight size="24px" />
        </Box>
      </HStack>
    </ChakraLink>
  );
}

export default ActivityListItem;
