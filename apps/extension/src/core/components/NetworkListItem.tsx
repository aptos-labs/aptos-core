// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React from 'react';
import {
  Box,
  Button,
  Center,
  HStack,
  Modal,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  ModalProps,
  Spinner,
  Text,
  useColorMode, useDisclosure,
  useRadio,
  UseRadioProps,
  VStack,
} from '@chakra-ui/react';
import { secondaryHoverBgColor, secondaryButtonColor } from 'core/colors';
import {
  Network, DefaultNetworks, defaultNetworks,
} from 'core/hooks/useGlobalState';
import { DeleteIcon } from '@chakra-ui/icons';

type ConfirmationModalProps = Omit<ModalProps, 'children'> & {
  name: string,
  onConfirm: () => void,
};

function ConfirmationModal(props: ConfirmationModalProps) {
  const { name, onClose, onConfirm } = props;

  return (
    <Modal {...props}>
      <ModalOverlay />
      <ModalContent>
        <ModalHeader>
          {`Are you sure you want to delete network '${name}'?`}
        </ModalHeader>
        <ModalCloseButton />
        <ModalFooter>
          <Button colorScheme="red" mr={3} onClick={onConfirm}>
            Yes
          </Button>
          <Button onClick={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

type NetworkListItemProps = UseRadioProps & {
  isLoading: boolean,
  network: Network,
  onRemove: (networkName: string) => void,
};

export default function NetworkListItem(props: NetworkListItemProps) {
  const { getCheckboxProps, getInputProps } = useRadio(props);
  const { isOpen, onClose, onOpen } = useDisclosure();
  const { colorMode } = useColorMode();
  const {
    isChecked, isDisabled, isLoading, network, onRemove, value,
  } = props;

  const isCustomNetwork = !(network.name in defaultNetworks);

  return (
    <Box as="label">
      <input disabled={isDisabled} {...getInputProps()} />
      <Box
        {...getCheckboxProps()}
        cursor="pointer"
        borderRadius="md"
        bgColor={secondaryButtonColor[colorMode]}
        _checked={{
          bg: 'teal.600',
          color: 'white',
        }}
        _hover={{
          bg: (isChecked) ? 'teal.700' : secondaryHoverBgColor[colorMode],
        }}
        _focus={{
          boxShadow: 'outline',
        }}
        px={5}
        py={3}
      >
        {
          !isLoading ? (
            <VStack alignItems="flex-start">
              <HStack w="100%" justifyContent="space-between">
                <Text fontSize="md" fontWeight={600}>
                  {network.name}
                </Text>
                {
                  isCustomNetwork ? (
                    <DeleteIcon
                      fontSize="lg"
                      cursor="pointer"
                      _hover={{
                        color: 'red.400',
                      }}
                      onClick={(e: React.MouseEvent) => {
                        e.preventDefault();
                        onOpen();
                      }}
                    />
                  ) : null
                }
              </HStack>
              <Text fontSize="md" fontWeight={400}>
                {network.nodeUrl}
              </Text>
              {
                (isDisabled && value === DefaultNetworks.LocalHost) ? (
                  <Text fontSize="sm">(Please start testnet and testnet faucet on localhost to switch)</Text>
                ) : undefined
              }
            </VStack>
          ) : (
            <Center>
              <Spinner />
            </Center>
          )
        }
      </Box>
      <ConfirmationModal
        isOpen={isOpen}
        onClose={onClose}
        onConfirm={() => onRemove(network.name)}
        name={network.name}
      />
    </Box>
  );
}
