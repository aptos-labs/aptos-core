// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from 'react';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
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
import { BsArrowCounterclockwise } from '@react-icons/all-files/bs/BsArrowCounterclockwise';

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

enum ActivityType {
  CoinReceived,
  CoinSent,
  ToSelf,
  Faucet,
}

interface TransactionDetails {
  recipient: string,
  sender: string
}

const activityDetailsMap = Object.freeze({
  [ActivityType.CoinReceived]: {
    amountColor: 'green.500',
    amountPrefix: '+',
    icon: <HiDownload />,
    text: ({ sender }: TransactionDetails) => `From ${collapseHexString(sender, 8)}`,
  },
  [ActivityType.CoinSent]: {
    amountColor: 'red.500',
    amountPrefix: '-',
    icon: <BsArrowUpRight />,
    text: ({ recipient }: TransactionDetails) => `To ${collapseHexString(recipient, 8)}`,
  },
  [ActivityType.ToSelf]: {
    amountColor: 'gray.500',
    amountPrefix: '',
    icon: <BsArrowCounterclockwise />,
    text: () => 'Sent to self',
  },
  [ActivityType.Faucet]: {
    amountColor: 'green.500',
    amountPrefix: '+',
    icon: <FontAwesomeIcon icon={faFaucet} />,
    text: () => 'Funded with Faucet',
  },
} as const);

function parseActivityType(activeAccountAddress: string, txn: Types.UserTransaction) {
  const payload = txn.payload as EntryFunctionPayload;
  const recipient = payload.arguments[0];
  if (txn.sender === recipient) {
    return ActivityType.ToSelf;
  }
  if (payload.function === '0x1::aptos_coin::mint') {
    return ActivityType.Faucet;
  }
  if (txn.sender === activeAccountAddress) {
    return ActivityType.CoinSent;
  }
  return ActivityType.CoinReceived;
}

interface ActivityItemProps {
  transaction: UserTransaction,
}

export function TransactionListItem({ transaction }: ActivityItemProps) {
  const { colorMode } = useColorMode();
  const { aptosAccount } = useActiveAccount();

  const payload = transaction.payload as EntryFunctionPayload;
  const { sender } = transaction;
  const [recipient, amount]: string[] = payload.arguments;
  const coinName = payload.type_arguments[0]?.split('::').pop();
  const formattedCoinName = formatCoinName(coinName);

  const myAddress = aptosAccount.address().toShortString();
  const activityType = parseActivityType(myAddress, transaction);
  const details = activityDetailsMap[activityType];

  const timestampMs = Number(transaction.timestamp) / 1000;
  const absDateTime = getAbsoluteDateTime(timestampMs);
  const relTime = useRelativeTime(timestampMs);

  const amountString = (formattedCoinName === APTOS_UNIT)
    ? `${details.amountPrefix}${formatCoin(BigInt(amount), { decimals: 8 })}`
    : `${details.amountPrefix}${numeral(amount).format('0,0')}`;

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
          { details.icon }
        </Circle>
        <VStack flexGrow={1} alignItems="start" spacing={0.5}>
          <HStack w="100%" fontSize="sm">
            <Text flexGrow={1}>
              { details.text({ recipient, sender }) }
            </Text>
            <Text
              maxWidth="45%"
              color={details.amountColor}
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

export default TransactionListItem;
