// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  Center,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { secondaryBorderColor } from 'core/colors';
import ActivityListItem from 'core/components/ActivityListItem';
import { Transaction } from 'shared/types/transaction';
import { formatAmount } from 'core/utils/coin';
import { BsArrowCounterclockwise } from '@react-icons/all-files/bs/BsArrowCounterclockwise';
import { BsArrowUpRight } from '@react-icons/all-files/bs/BsArrowUpRight';
import collapseHexString from 'core/utils/hex';
import { HiDownload } from '@react-icons/all-files/hi/HiDownload';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import { useActiveAccount } from 'core/hooks/useAccounts';

function NoActivity() {
  const { colorMode } = useColorMode();
  return (
    <Box w="100%" borderWidth="1px" borderRadius=".5rem" borderColor={secondaryBorderColor[colorMode]}>
      <Center height="100%" p={4}>
        <Text fontSize="md" textAlign="center">No activity yet!</Text>
      </Center>
    </Box>
  );
}

const positiveAmountColor = 'green.500';
const negativeAmountColor = 'red.500';
const neutralAmountColor = 'gray.500';
const formatAmountOptions = { decimals: 4, prefix: true } as const;
const coinDepositIcon = <HiDownload />;
const coinWithdrawalIcon = <BsArrowUpRight />;
const selfTransferIcon = <BsArrowCounterclockwise />;
const coinMintIcon = <FontAwesomeIcon icon={faFaucet} />;

function extractActivityItemsFromTransaction(
  activeAccountAddress: string,
  txn: Transaction,
) {
  const common = {
    key: `${txn.version}`,
    timestamp: txn.timestamp,
    txnVersion: txn.version,
  };

  if (txn.type === 'transfer') {
    const wereCoinsSent = activeAccountAddress === txn.sender;
    const wereCoinsReceived = activeAccountAddress === txn.recipient;

    if (wereCoinsSent && wereCoinsReceived) {
      return [{
        amount: formatAmount(txn.amount, txn.coinInfo, {
          ...formatAmountOptions,
          prefix: false,
        }),
        amountColor: neutralAmountColor,
        icon: selfTransferIcon,
        text: 'Sent to self',
        ...common,
      }];
    }

    if (wereCoinsSent) {
      return [{
        amount: formatAmount(-txn.amount, txn.coinInfo, formatAmountOptions),
        amountColor: negativeAmountColor,
        icon: coinWithdrawalIcon,
        text: `To ${collapseHexString(txn.recipient, 8)}`,
        ...common,
      }];
    }

    return [{
      amount: formatAmount(txn.amount, txn.coinInfo, formatAmountOptions),
      amountColor: positiveAmountColor,
      icon: coinDepositIcon,
      text: `From ${collapseHexString(txn.sender, 8)}`,
      ...common,
    }];
  }

  if (txn.type === 'mint') {
    return [{
      amount: formatAmount(txn.amount, txn.coinInfo, formatAmountOptions),
      amountColor: positiveAmountColor,
      icon: coinMintIcon,
      text: 'Funded with Faucet',
      ...common,
    }];
  }

  return Object.values(txn.coinBalanceChanges[activeAccountAddress])
    .flatMap(({ amount, coinInfo }, index) => {
      const amountColor = amount > 0 ? positiveAmountColor : negativeAmountColor;
      const icon = amount > 0 ? coinDepositIcon : coinWithdrawalIcon;
      const text = amount > 0 ? 'Deposited' : 'Withdrawn';
      return [{
        ...common,
        amount: formatAmount(amount, coinInfo, formatAmountOptions),
        amountColor,
        icon,
        key: `${common.key}_${index}`,
        text,
      }];
    });
}

interface ActivityListProps {
  transactions: Transaction[] | undefined,
}

export function ActivityList({
  transactions,
}: ActivityListProps) {
  const { activeAccountAddress } = useActiveAccount();
  const activityItems = transactions?.flatMap((txn) => extractActivityItemsFromTransaction(
    activeAccountAddress,
    txn,
  ));

  const hasActivity = activityItems !== undefined && activityItems.length > 0;
  return (
    <VStack w="100%" spacing={3}>
      {
        hasActivity
          ? activityItems.map(({ key, ...props }) => <ActivityListItem key={key} {...props} />)
          : <NoActivity />
      }
    </VStack>
  );
}

export default ActivityList;
