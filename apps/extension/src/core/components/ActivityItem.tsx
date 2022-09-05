// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo, useState } from 'react';
import {
  Circle,
  HStack,
  Text,
  Tooltip,
  useColorMode,
  useInterval,
  VStack,
} from '@chakra-ui/react';
import { HiDownload } from '@react-icons/all-files/hi/HiDownload';
import { BsArrowUpRight } from '@react-icons/all-files/bs/BsArrowUpRight';
import { Types } from 'aptos';
import ChakraLink from 'core/components/ChakraLink';
import { collapseHexString } from 'core/utils/hex';
import {
  secondaryGridBgColor,
  secondaryGridHoverBgColor,
  timestampColor,
} from 'core/colors';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { formatCoinName } from 'core/hooks/useTransactionDetails';
import { APTOS_UNIT, formatCoin } from 'core/utils/coin';
import numeral from 'numeral';

type EntryFunctionPayload = Types.EntryFunctionPayload;
type UserTransaction = Types.UserTransaction;

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

interface ActivityItemProps {
  transaction: UserTransaction,
}

export function ActivityItem({ transaction }: ActivityItemProps) {
  const { colorMode } = useColorMode();
  const { aptosAccount } = useActiveAccount();

  const typedPayload = transaction.payload as EntryFunctionPayload;
  const [recipient, amount]: string[] = typedPayload.arguments;
  const coinName = typedPayload.type_arguments[0]?.split('::').pop();
  const formattedCoinName = useMemo(() => formatCoinName(coinName), [coinName]);

  const myAddress = aptosAccount.address().toShortString();
  const isSent = myAddress === transaction.sender;
  const otherAddress = isSent ? recipient : transaction.sender;

  const timestampMs = Number(transaction.timestamp) / 1000;
  const absDateTime = getAbsoluteDateTime(timestampMs);
  const relTime = useRelativeTime(timestampMs);

  const isSentPrefix = isSent ? '-' : '+';
  const amountString = (formattedCoinName === APTOS_UNIT)
    ? `${isSentPrefix}${formatCoin(Number(amount), { decimals: 8 })}`
    : `${isSentPrefix}${numeral(amount).format('0,0')}`;

  return (
    <ChakraLink to={`/transactions/${transaction.version}`} w="100%">
      <HStack
        spacing={4}
        padding={3}
        paddingLeft={4}
        paddingRight={4}
        cursor="pointer"
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
              { collapseHexString(otherAddress, 8) }
            </Text>
            <Text
              maxWidth="45%"
              color={isSent ? 'red.500' : 'green.500'}
              fontWeight={500}
              whiteSpace="nowrap"
              overflow="hidden"
              textOverflow="ellipsis"
            >
              {amountString }
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

export default ActivityItem;
