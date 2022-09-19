// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useRef, useState } from 'react';
import WalletLayout from 'core/layouts/WalletLayout';
import {
  VStack,
  Text,
  Flex,
  Box,
  useColorMode,
  Textarea,
  HStack,
  Icon,
  Button,
} from '@chakra-ui/react';
import { Transition, type TransitionStatus } from 'react-transition-group';
import { RiFileCopyLine } from '@react-icons/all-files/ri/RiFileCopyLine';
import { useActiveAccount } from 'core/hooks/useAccounts';
import Copyable from 'core/components/Copyable';
import {
  secondaryTextColor,
  secondaryBorderColor,
  customColors,
  rotationKeyButtonBgColor,
} from 'core/colors';
import ConfirmationPopup from 'core/components/ConfirmationPopup';
import { useNavigate } from 'react-router-dom';
import { Routes } from 'core/routes';
import useRotateKey from 'core/hooks/useRotateKey';
import { RiErrorWarningFill } from '@react-icons/all-files/ri/RiErrorWarningFill';

const transitionDuration = 200;

function WarningLogo() {
  return (
    <Box bgColor="rgba(243, 168, 69, 0.1)" borderRadius={100} width="75px" height="75px" display="flex" justifyContent="center" alignItems="center">
      <RiErrorWarningFill size={36} color="#F3A845" />
    </Box>
  );
}

export default function RotateKey() {
  const { colorMode } = useColorMode();
  const ref = useRef(null);
  const [hasRotatedKey, setHasRotatedKey] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [showPopup, setShowPopup] = useState<boolean>(false);
  const { activeAccount } = useActiveAccount();
  const navigate = useNavigate();
  const { rotateKey } = useRotateKey();

  const onRotateKeyStart = () => setIsLoading(true);
  const onRotateKeySuccess = () => setHasRotatedKey(true);
  const onRotateKeyComplete = () => {
    setShowPopup(false);
    setIsLoading(false);
  };

  const handleRotateKey = async () => {
    onRotateKeyStart();
    await rotateKey({
      onRotateKeyComplete,
      onRotateKeySuccess,
    });
    onRotateKeyComplete();
  };

  return (
    <Box width="100%" height="100%" position="relative">
      <WalletLayout title="Rotate Key" showBackButton showAccountCircle={false} hasWalletFooter={false}>
        <Flex width="100%" height="100%" flexDirection="column">
          <Box width="100%" mt={4} flex={1} px={4}>
            <Flex>
              <Text
                fontSize="md"
                fontWeight={700}
                flex={1}
              >
                {hasRotatedKey ? 'New private key' : 'Rotate your private key'}
              </Text>
              <Copyable
                prompt="Copy private key"
                value={activeAccount.privateKey}
              >
                <HStack alignItems="center" height="100%">
                  <Text
                    fontSize="md"
                    fontWeight={500}
                    textAlign="center"
                  >
                    Copy
                  </Text>
                  <Icon as={RiFileCopyLine} my="auto" w={3} h={3} margin="auto" />
                </HStack>
              </Copyable>
            </Flex>
            <Textarea
              marginTop={4}
              color={secondaryTextColor[colorMode]}
              height={18}
              readOnly
              variant="filled"
              fontSize="md"
              value={activeAccount.privateKey}
            />
          </Box>
          <Box borderTop="1px" px={4} mt={4} width="100%" borderColor={secondaryBorderColor[colorMode]} paddingTop={4}>
            {!hasRotatedKey
              ? (
                <Button
                  py={6}
                  width="100%"
                  colorScheme="salmon"
                  color="white"
                  onClick={() => setShowPopup(true)}
                >
                  Rotate
                </Button>
              ) : null}
            {hasRotatedKey
              ? (
                <VStack width="100%" spacing={2}>
                  <Copyable
                    prompt="Copy private key"
                    value={activeAccount.privateKey}
                    as="div"
                    width="100%"
                  >
                    <Button
                      width="100%"
                      bgColor={rotationKeyButtonBgColor[colorMode]}
                      border="1px"
                      borderColor={customColors.navy[200]}
                    >
                      Copy
                    </Button>
                  </Copyable>
                  <Button
                    width="100%"
                    colorScheme="salmon"
                    color="white"
                    onClick={() => navigate(
                      Routes.manage_account_show_recovery_phrase.path,
                      { state: { hasRotatedKey } },
                    )}
                  >
                    Next
                  </Button>
                </VStack>
              ) : null}
          </Box>
        </Flex>
      </WalletLayout>
      <Transition in={showPopup} timeout={transitionDuration} nodeRef={ref}>
        {(state: TransitionStatus) => (
          <ConfirmationPopup
            bodyWidth="320px"
            open={showPopup}
            duration={transitionDuration}
            state={state}
            logo={<WarningLogo />}
            isLoading={isLoading}
            title="Are you sure?"
            body="Rotating your private key will generate a new secret recover phrase."
            primaryBttnLabel="Yes, rotate key"
            primaryBttnOnClick={async () => {
              await handleRotateKey();
            }}
            secondaryBttnLabel="Cancel"
            secondaryBttnOnClick={() => {
              setShowPopup(false);
            }}
          />
        )}
      </Transition>
    </Box>
  );
}
