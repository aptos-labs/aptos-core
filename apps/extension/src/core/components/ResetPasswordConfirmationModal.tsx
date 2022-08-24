import React from 'react';

import {
  Button,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalFooter,
  ModalHeader,
  ModalOverlay,
  ModalProps,
  Text,
} from '@chakra-ui/react';

type ConfirmationModalProps = Omit<ModalProps, 'children'> & {
  onConfirm: () => void;
};

export default function ResetPasswordConfirmationModal(props: ConfirmationModalProps) {
  const { onClose, onConfirm } = props;

  return (
    <Modal {...props}>
      <ModalOverlay />
      <ModalContent>
        <ModalHeader>Are you sure you want to reset the password?</ModalHeader>
        <ModalCloseButton />
        <ModalBody>
          <Text fontSize="sm">
            PLEASE NOTE: You will not be able to recover your wallet account
            unless you have stored the private key or mnemonic associated with
            your wallet address.
          </Text>
        </ModalBody>
        <ModalFooter>
          <Button colorScheme="red" mr={3} onClick={onConfirm}>
            Yes, I understand
          </Button>
          <Button onClick={onClose}>Close</Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
