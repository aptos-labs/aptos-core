// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  ChevronDownIcon,
  ChevronUpIcon,
} from '@chakra-ui/icons';
import {
  Box,
  Button,
  Collapse,
  Heading,
  Text,
  VStack,
  chakra,
  useColorMode,
  useDisclosure,
} from '@chakra-ui/react';
import React, { useEffect, useState } from 'react';
import { useQueryClient } from 'react-query';
import { Types } from 'aptos';
import SyntaxHighlighter from 'react-syntax-highlighter';
import { darcula, docco } from 'react-syntax-highlighter/dist/esm/styles/hljs';

import { secondaryErrorMessageColor } from 'core/colors';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useTransactionSimulation } from 'core/hooks/useTransactions';
import { useAccountOctaCoinBalance } from 'core/queries/account';
import { bigIntMin } from 'core/utils/bigint';
import { formatAmount, formatCoin } from 'core/utils/coin';
import { maxGasFeeFromEstimated } from 'shared/transactions';
import { CoinBalanceChangesByCoinType, Transaction } from 'shared/types';
import { usePermissionRequestContext } from '../hooks';
import { LoadableContent } from './LoadableContent';
import { PermissionPromptLayout } from './PermissionPromptLayout';
import { Tile } from './Tile';

const ChakraSyntaxHighlighter = chakra(SyntaxHighlighter);

const simulationQueryKey = 'promptSimulation';
const simulationRefetchInterval = 10000;

const positiveAmountColor = 'green.500';
const negativeAmountColor = 'red.500';

const headingProps = {
  lineHeight: '24px',
  mb: '4px',
  size: 'sm',
};

// region Components

interface TransactionCoinBalanceChangeProps {
  changes: CoinBalanceChangesByCoinType,
}

/**
 * Display changes relative to a coin store
 * @param coinStore coin store changes
 */
function TransactionCoinBalanceChange({ changes }: TransactionCoinBalanceChangeProps) {
  return (
    <Box>
      <Heading {...headingProps}>
        Estimated Balance Changes
      </Heading>
      {
        Object.values(changes).map(({ amount, coinInfo }) => (
          <Text fontSize="sm" lineHeight="20px">
            Amount:&nbsp;
            <Text as="span" color={amount > 0 ? positiveAmountColor : negativeAmountColor}>
              { formatAmount(amount, coinInfo) }
            </Text>
          </Text>
        ))
      }
    </Box>
  );
}

interface TransactionWritesetProps {
  txn: Transaction,
}

function TransactionWriteset({ txn }: TransactionWritesetProps) {
  const { isOpen, onToggle } = useDisclosure();
  const { colorMode } = useColorMode();
  const style = colorMode === 'light' ? docco : darcula;

  return (
    <Box>
      <Button size="xs" variant="unstyled" onClick={onToggle}>
        <Heading {...headingProps}>
          Raw writeset
          { isOpen ? <ChevronDownIcon ml={1} /> : <ChevronUpIcon ml={1} /> }
        </Heading>
      </Button>
      <Collapse startingHeight={0} in={isOpen}>
        <ChakraSyntaxHighlighter
          w="100%"
          h="300px"
          mt={1}
          language="json"
          style={style}
        >
          { JSON.stringify(txn.rawChanges, null, 4) }
        </ChakraSyntaxHighlighter>
      </Collapse>
    </Box>
  );
}

// endregion

interface TransactionApprovalPromptProps {
  payload: Types.EntryFunctionPayload,
}

export function TransactionApprovalPrompt({ payload }: TransactionApprovalPromptProps) {
  const { colorMode } = useColorMode();
  const queryClient = useQueryClient();
  const { activeAccountAddress } = useActiveAccount();
  const { data: coinBalance } = useAccountOctaCoinBalance(activeAccountAddress, {
    refetchInterval: simulationRefetchInterval,
  });

  const maxGas = coinBalance
    ? Number(bigIntMin(coinBalance, BigInt(Number.MAX_SAFE_INTEGER)))
    : undefined;
  const simulation = useTransactionSimulation(simulationQueryKey, payload, {
    cacheTime: 0,
    enabled: coinBalance !== undefined,
    maxGasOctaAmount: maxGas,
    refetchInterval: simulationRefetchInterval,
  });
  const { setApprovalState } = usePermissionRequestContext();

  // Keeping error state, since the query error is cleared on re-fetching
  const [simulationError, setSimulationError] = useState<string | undefined>();
  useEffect(() => {
    queryClient.resetQueries(simulationQueryKey).then();
  }, [payload, queryClient]);

  useEffect(() => {
    if (simulation.error) {
      setSimulationError(simulation.error.message);
    } else if (simulation.data?.error !== undefined) {
      setSimulationError(simulation.data.error.reasonDescr);
    } else if (!simulation.isLoading) {
      setSimulationError(undefined);
    }
  }, [simulation.data, simulation.error, simulation.isLoading]);

  useEffect(() => {
    const approvalArgs = simulation.data && simulation.data.success
      ? {
        gasUnitPrice: simulation.data.gasUnitPrice,
        maxGasFee: maxGasFeeFromEstimated(Number(simulation.data.gasFee)),
      }
      : undefined;
    setApprovalState({ args: approvalArgs, canApprove: approvalArgs !== undefined });
  }, [simulation.data, setApprovalState]);

  const networkFee = simulation.data !== undefined
    ? formatCoin(simulation.data.gasFee * simulation.data.gasUnitPrice, { decimals: 8 })
    : undefined;

  const hasCoinBalanceChanges = simulation.data !== undefined
    && simulation.data.coinBalanceChanges[activeAccountAddress]
    && Object.keys(simulation.data.coinBalanceChanges[activeAccountAddress]).length > 0;
  const ownCoinBalanceChanges = hasCoinBalanceChanges
    ? simulation.data.coinBalanceChanges[activeAccountAddress]
    : undefined;

  return (
    <PermissionPromptLayout title="Approve transaction">
      <Tile>
        <LoadableContent isLoading={simulation.isLoading}>
          <VStack spacing="21px" alignItems="stretch">
            <Box>
              <Heading {...headingProps}>
                Payload
              </Heading>
              <Text fontSize="sm" lineHeight="20px">
                {`Function: ${payload.function}`}
              </Text>
              { /* TODO: use ABI to retrieve arguments names */}
            </Box>
            {
              simulationError
                ? (
                  <Box color={secondaryErrorMessageColor[colorMode]}>
                    <Heading {...headingProps}>
                      Transaction error
                    </Heading>
                    <Text fontSize="sm" lineHeight="20px">
                      {simulationError}
                    </Text>
                  </Box>
                )
                : null
            }
            {
              !simulationError && networkFee !== undefined
                ? (
                  <Box>
                    <Heading {...headingProps}>
                      Network Fee
                    </Heading>
                    <Text fontSize="sm" lineHeight="20px">
                      { networkFee }
                    </Text>
                  </Box>
                )
                : null
            }
            {
              !simulationError && ownCoinBalanceChanges !== undefined
                ? <TransactionCoinBalanceChange changes={ownCoinBalanceChanges} />
                : null
            }
            {
              !simulationError && simulation.data !== undefined
                ? <TransactionWriteset txn={simulation.data} />
                : null
            }
          </VStack>
        </LoadableContent>
      </Tile>
    </PermissionPromptLayout>
  );
}

export default TransactionApprovalPrompt;
