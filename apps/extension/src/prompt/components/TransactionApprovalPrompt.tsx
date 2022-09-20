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
import { HexString, Types } from 'aptos';
import SyntaxHighlighter from 'react-syntax-highlighter';
import { darcula, docco } from 'react-syntax-highlighter/dist/esm/styles/hljs';

import { secondaryErrorMessageColor } from 'core/colors';
import { aptosCoinStoreStructTag } from 'core/constants';
import { useActiveAccount } from 'core/hooks/useAccounts';
import { useTransactionSimulation } from 'core/hooks/useTransactions';
import { useAccountOctaCoinBalance } from 'core/queries/account';
import { formatCoin } from 'core/utils/coin';
import { maxGasFeeFromEstimated } from 'shared/transactions';
import { parseMoveAbortDetails } from 'shared/move';
import { usePermissionRequestContext } from '../hooks';
import { LoadableContent } from './LoadableContent';
import { PermissionPromptLayout } from './PermissionPromptLayout';
import { Tile } from './Tile';

const ChakraSyntaxHighlighter = chakra(SyntaxHighlighter);

const simulationQueryKey = 'promptSimulation';
const simulationRefetchInterval = 10000;

const headingProps = {
  lineHeight: '24px',
  mb: '4px',
  size: 'sm',
};

// region Utils

/**
 * Convert number to little endian hex representation.
 * Used to compute the first part of an event key.
 * @param value numeric value to convert
 * @param digits number of hex digits
 */
function toLittleEndianHex(value: number, digits = 16) {
  const words = value.toString(digits).padStart(digits, '0').match(/.{2}/g);
  return words!.reverse().join('');
}

/**
 * Retrieve an event key in the format `0x{creationNum}{address}` from an event state
 * @param eventState event state obtained from a writeset
 */
function getEventKeyFromState(eventState: any) {
  const resourceAddr = new HexString(eventState.guid.id.addr);
  const creationNum = Number(eventState.guid.id.creation_num);
  return `0x${toLittleEndianHex(creationNum)}${resourceAddr.noPrefix()}`;
}

/**
 * Get coin store changes from the associated state and events
 * @param coinStoreState state obtained from a writeset
 * @param events events emitted during the transaction
 */
function getCoinStoreChanges(coinStoreState: any, events: Types.Event[]) {
  // TODO: convert to bigint
  const newBalance = coinStoreState?.coin?.value !== undefined
    ? Number(coinStoreState.coin.value)
    : undefined;

  const withdrawEventKey = getEventKeyFromState(coinStoreState.withdraw_events);
  const depositEventKey = getEventKeyFromState(coinStoreState.deposit_events);

  let balanceChange = 0;
  events.forEach(({ data, key }) => {
    if (key === depositEventKey) {
      balanceChange += Number(data.amount);
    } else if (key === withdrawEventKey) {
      balanceChange -= Number(data.amount);
    }
  });

  return {
    balanceChange,
    newBalance,
  };
}

/**
 * Get useful details to be shown to the user after a transaction simulation
 * @param txn simulation used to extract info
 */
function getSimulationDetails(txn: Types.UserTransaction) {
  // No matter the type of transaction, the gas fee is always useful
  const networkFee = Number(txn.gas_used) * Number(txn.gas_unit_price);

  // Should always display changes in the user's resources
  const ownResourceWrites = txn.changes
    .filter((change) => change.type === 'write_resource')
    .map((change) => change as Types.WriteResource)
    .filter((change) => change.address === txn.sender);

  const newAptosCoinStoreState = ownResourceWrites
    .find((write) => write.data.type === aptosCoinStoreStructTag)
    ?.data.data as any;

  const aptosCoinStore = newAptosCoinStoreState !== undefined
    ? getCoinStoreChanges(newAptosCoinStoreState, txn.events)
    : undefined;

  return {
    aptosCoinStore,
    networkFee,
  };
}

type CoinStoreChanges = ReturnType<typeof getCoinStoreChanges>;

// endregion

// region Components

interface TransactionCoinBalanceChangeProps {
  coinStore: CoinStoreChanges,
}

/**
 * Display changes relative to a coin store
 * @param coinStore coin store changes
 */
function TransactionCoinBalanceChange({ coinStore }: TransactionCoinBalanceChangeProps) {
  return (
    <Box>
      <Heading {...headingProps}>
        Estimated Balance Changes
      </Heading>
      <Text fontSize="sm" lineHeight="20px">
        Amount:&nbsp;
        <Text as="span" color="teal">
          {formatCoin(coinStore.balanceChange, { decimals: 8 })}
        </Text>
      </Text>
      {/* <Text fontSize="sm" lineHeight="20px">
        New balance:&nbsp;
        <Text as="span" color="teal">
          {formatCoin(coinStore.newBalance, { decimals: 8 })}
        </Text>
      </Text> */}
    </Box>
  );
}

interface TransactionWritesetProps {
  txn: Types.UserTransaction,
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
          { JSON.stringify(txn.changes, null, 4) }
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
  const simulation = useTransactionSimulation(simulationQueryKey, payload, {
    cacheTime: 0,
    enabled: coinBalance !== undefined,
    maxGasOctaAmount: coinBalance,
    refetchInterval: simulationRefetchInterval,
  });
  const { setApprovalState } = usePermissionRequestContext();
  const details = simulation.data && getSimulationDetails(simulation.data);

  const [simulationError, setSimulationError] = useState<string | undefined>();
  useEffect(() => {
    queryClient.resetQueries(simulationQueryKey).then();
  }, [payload, queryClient]);

  useEffect(() => {
    if (simulation.error) {
      setSimulationError(simulation.error.message);
    } else if (simulation.data?.success === false) {
      const abortDetails = parseMoveAbortDetails(simulation.data.vm_status);
      setSimulationError(abortDetails?.reasonDescr);
    } else if (!simulation.isLoading) {
      setSimulationError(undefined);
    }
  }, [simulation.data, simulation.error, simulation.isLoading]);

  useEffect(() => {
    const isSimulationSuccessful = simulation.data?.success === true;
    const approvalArgs = isSimulationSuccessful
      ? {
        gasUnitPrice: Number(simulation.data!.gas_unit_price),
        maxGasFee: maxGasFeeFromEstimated(Number(simulation.data!.gas_used)),
      }
      : undefined;
    setApprovalState({ args: approvalArgs, canApprove: isSimulationSuccessful });
  }, [simulation.data, setApprovalState]);

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
              !simulationError && details?.networkFee !== undefined
                ? (
                  <Box>
                    <Heading {...headingProps}>
                      Network Fee
                    </Heading>
                    <Text fontSize="sm" lineHeight="20px">
                      {formatCoin(details.networkFee, { decimals: 8 })}
                    </Text>
                  </Box>
                )
                : null
            }
            {
              !simulationError && details?.aptosCoinStore !== undefined
                ? <TransactionCoinBalanceChange coinStore={details.aptosCoinStore} />
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
