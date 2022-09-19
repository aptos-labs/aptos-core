// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, {
  useMemo, useRef, LegacyRef, forwardRef,
} from 'react';
import {
  Box, Center, Text, Button,
  useColorMode,
} from '@chakra-ui/react';
import type { TransitionStatus } from 'react-transition-group';
import { customColors, bgColorButtonPopup } from 'core/colors';

const bgColorOverlay = {
  dark: 'rgba(191, 191, 191, 0.5)',
  light: 'rgba(171, 183, 183, 0.5)',
};

const bgColorPopup = {
  dark: 'gray.800',
  light: 'white',
};

interface ConfirmationPopupProps {
  body: string;
  bodyWidth: string;
  duration: number;
  isLoading?: boolean;
  logo: React.ReactNode;
  open: boolean;
  primaryBttnLabel: string;
  primaryBttnOnClick: () => void;
  secondaryBttnLabel: string;
  secondaryBttnOnClick: () => void;
  state: TransitionStatus;
  title: string;
}

interface BackdropProps {
  children: JSX.Element;
  duration: number;
  state: TransitionStatus;
}

const ConfirmationBackdrop = forwardRef(
  ({
    children, duration, state,
  }: BackdropProps, ref) => {
    const { colorMode } = useColorMode();

    const defaultStyle = useMemo(() => ({
      backgroundColor: 'rgba(0, 0, 0, 0)',
      transition: `background-color ${duration}ms ease-in-out, z-index ${duration}ms ease-in-out`,
      zIndex: -2,
    }), [duration]);

    const transitionStyles = useMemo(() => ({
      entered: { backgroundColor: 'rgba(0, 0, 0, 0.5)', zIndex: 2 },
      entering: { backgroundColor: 'rgba(0, 0, 0, 0)', zIndex: -2 },
      exited: {},
      exiting: {},
      unmounted: {},
    }), []);

    return (
      <Box
        ref={ref as LegacyRef<HTMLDivElement>}
        className="modal-backdrop"
        display="flex"
        flexDirection="column-reverse"
        width="375px"
        height="600px"
        position="absolute"
        top={0}
        bottom={0}
        left={0}
        right={0}
        zIndex="2"
        mt={0}
        bgColor={bgColorOverlay[colorMode]}
        style={{
          ...defaultStyle,
          ...transitionStyles[state],
        }}
      >
        {children}
      </Box>
    );
  },
);

function ConfirmationPopup({
  body,
  bodyWidth,
  duration,
  isLoading,
  logo,
  open,
  primaryBttnLabel,
  primaryBttnOnClick,
  secondaryBttnLabel,
  secondaryBttnOnClick,
  state,
  title,
}: ConfirmationPopupProps) {
  const { colorMode } = useColorMode();
  const ref = useRef(null);

  const defaultStyle = useMemo(() => ({
    transform: 'translateY(100%)',
    transition: `transform ${duration}ms ease-in-out`,
  }), [duration]);

  const transitionStyles = useMemo(() => ({
    entered: { transform: 'translateY(0)' },
    entering: { transform: 'translateY(100%)' },
    exited: {},
    exiting: {},
    unmounted: {},
  }), []);

  return (
    <ConfirmationBackdrop
      ref={ref}
      duration={duration}
      state={state}
    >
      <Box
        flexDirection="column"
        bgColor={bgColorPopup[colorMode]}
        alignContent="flex-end"
        height="60%"
        p={4}
        borderTopRadius={8}
        opacity={1}
        display="flex"
        justifyContent="center"
        style={{
          ...defaultStyle,
          ...transitionStyles[state],
        }}
      >
        {open && (
        <>
          <Center mx="auto" mb={4} display="flex" flexDirection="column" height="100%" width={bodyWidth || '260px'} justifyContent="center" flex="1">
            <Box pb={4} width="100%" display="flex" flexDirection="column" justifyContent="center" textAlign="center">
              <Center pb={4} width="100%" height="100%">
                {logo}
              </Center>
              <Text
                fontSize={23}
                fontWeight="bold"
              >
                {title}
              </Text>
            </Box>
            <Box width="100%" display="flex" textAlign="center">
              <Text
                fontSize={16}
              >
                {body}
              </Text>
            </Box>
          </Center>
          <Box key="button" width="100%" display="flex" flexDirection="column" gap={3}>
            <Button
              key={secondaryBttnLabel}
              bgColor={bgColorButtonPopup[colorMode]}
              border="1px"
              borderColor={customColors.navy[300]}
              height="48px"
              isDisabled={isLoading || false}
              variant="solid"
              width="100%"
              onClick={secondaryBttnOnClick}
            >
              {secondaryBttnLabel}
            </Button>
            <Button
              key={primaryBttnLabel}
              colorScheme="salmon"
              height="48px"
              color="white"
              isLoading={isLoading || false}
              variant="solid"
              width="100%"
              onClick={primaryBttnOnClick}
            >
              {primaryBttnLabel}
            </Button>
          </Box>
        </>
        )}
      </Box>
    </ConfirmationBackdrop>
  );
}

export default ConfirmationPopup;
