/* eslint-disable @typescript-eslint/no-unused-vars */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Box, Button, Center, Flex, Heading, SimpleGrid, Spinner, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import { secondaryAddressFontColor, secondaryBorderColor, secondaryWalletHomeCardBgColor } from 'core/colors';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { type StakeInfo, useAccountStakeInfo } from 'core/queries/account';
import React, { useMemo } from 'react';
import formatDuration from 'date-fns/formatDuration';
import intervalToDuration from 'date-fns/intervalToDuration';
import addSeconds from 'date-fns/addSeconds';
import format from 'date-fns/format';
import { type Duration } from 'date-fns';
import numeral from 'numeral';
import { collapseHexString } from 'core/utils/hex';
import { ChevronRightIcon } from '@chakra-ui/icons';
import { AptosLogo } from './AptosLogo';
import ChakraLink from './ChakraLink';
import Copyable from './Copyable';

interface DurationStringFormatProps {
  duration: Duration;
}

const durationFormat = Object.freeze([
  'years', 'months', 'weeks', 'days', 'hours', 'minutes', 'seconds',
] as const);

const durationFormatBoundary = ({
  duration,
}: DurationStringFormatProps) => {
  if (duration.years && duration.years > 0) {
    return durationFormat.slice(0, 4);
  }
  if (duration.months && duration.months > 0) {
    return durationFormat.slice(1, 4);
  }
  if (duration.weeks && duration.weeks > 0) {
    return durationFormat.slice(2, 5);
  }
  if (duration.days && duration.days > 0) {
    return durationFormat.slice(3, 6);
  }
  if (duration.hours && duration.hours > 0) {
    return durationFormat.slice(4, 7);
  }
  return durationFormat.slice(5);
};

interface NoStakeProps {
  isLoading: boolean;
}

function NoStake({
  isLoading,
}: NoStakeProps) {
  const { colorMode } = useColorMode();
  return (
    <Box w="100%" borderWidth="1px" borderRadius=".5rem" borderColor={secondaryBorderColor[colorMode]}>
      <Center height="100%" p={4}>
        { isLoading ? (
          <Spinner />
        ) : (
          <Text fontSize="md" textAlign="center">No stake yet!</Text>
        )}
      </Center>
    </Box>
  );
}
interface StakeBodyContentProps {
  stakeInfo: StakeInfo
}

function StakeBodyContent({
  stakeInfo,
}: StakeBodyContentProps) {
  const { colorMode } = useColorMode();
  const {
    delegatedVoter,
    lockedUntilSecs,
    operatorAddress,
    value: stakeAmount,
  } = stakeInfo;

  const stakeAmountString = numeral(stakeAmount).format('0,0');

  const formattedDuration = useMemo(() => {
    const currDate = new Date();
    const lockedUntilDate = addSeconds(currDate, Number(lockedUntilSecs));
    const duration = intervalToDuration({
      end: lockedUntilDate,
      start: currDate,
    });
    const boundary = durationFormatBoundary({ duration });

    const durationString = formatDuration(
      duration,
      { format: boundary, zero: true },
    );
    return durationString;
  }, [lockedUntilSecs]);

  const lockedUntilDateString = useMemo(() => {
    const currDate = new Date();
    const lockedUntilDate = addSeconds(currDate, Number(lockedUntilSecs));
    return format(lockedUntilDate, 'MMM dd yyyy');
  }, [lockedUntilSecs]);

  const stakingAmount = (
    <VStack
      py={4}
      width="100%"
      px={4}
      borderRadius=".5rem"
      bgColor={secondaryWalletHomeCardBgColor[colorMode]}
      alignItems="flex-start"
    >
      <SimpleGrid columns={2} width="100%">
        <Box width="72px" height="72px">
          <AptosLogo />
        </Box>
      </SimpleGrid>
      <Text pt={4} fontSize="sm" color={secondaryAddressFontColor[colorMode]}>My stake</Text>
      <span>
        <Heading as="span" wordBreak="break-word" maxW="100%">{stakeAmountString}</Heading>
        <Text pl={2} pb="2px" as="span" fontSize="xl" fontWeight={600}>
          APT
        </Text>
      </span>
      <Text color={secondaryAddressFontColor[colorMode]} fontSize="sm">
        Locked until
        {' '}
        {lockedUntilDateString}
      </Text>
    </VStack>
  );

  const voter = (
    <ChakraLink to={`/accounts/${delegatedVoter}`} w="100%">
      <Button
        py={10}
        width="100%"
        rightIcon={<ChevronRightIcon />}
        justifyContent="space-between"
      >
        <VStack
          py={4}
          width="100%"
          borderRadius=".5rem"
          alignItems="flex-start"
        >
          <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>Voter</Text>
          <Copyable prompt="Copy address" value={delegatedVoter}>
            <Text fontSize="md">
              {collapseHexString(delegatedVoter)}
            </Text>
          </Copyable>
        </VStack>
      </Button>
    </ChakraLink>

  );

  const operator = (
    <ChakraLink to={`/accounts/${delegatedVoter}`} w="100%">
      <Button
        py={10}
        width="100%"
        rightIcon={<ChevronRightIcon />}
        justifyContent="space-between"
      >
        <VStack
          py={4}
          width="100%"
          borderRadius=".5rem"
          alignItems="flex-start"
        >
          <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>Operator</Text>
          <Copyable prompt="Copy address" value={delegatedVoter}>

            <Text fontSize="md">
              {collapseHexString(operatorAddress)}
            </Text>
          </Copyable>
        </VStack>
      </Button>
    </ChakraLink>

  );

  return (
    <VStack width="100%">
      {stakingAmount}
      {voter}
      {operator}
    </VStack>

  );
}

export default function StakeBody() {
  const { activeAccountAddress } = useActiveAccount();
  // TODO: change back to activeAccountAddress
  const {
    data: stakeInfo,
    isError,
    isLoading,
  } = useAccountStakeInfo(
    '0xb77026ce272a63b7261d20e5d0d9ca8cddd81b42b3432891668c43c03dbd1b73',
    {
      refetchInterval: 5000,
    },
  );

  return (
    <VStack width="100%" paddingTop={(stakeInfo) ? 4 : 8} alignItems="start">
      {
        (stakeInfo && !isLoading && !isError) ? (
          <StakeBodyContent stakeInfo={stakeInfo} />
        ) : <NoStake isLoading={isLoading} />
      }
    </VStack>
  );
}
