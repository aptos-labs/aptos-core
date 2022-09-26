// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, {
  useMemo, useRef, LegacyRef, forwardRef,
} from 'react';
import {
  Box, Center, Text, Button,
  useColorMode,
} from '@chakra-ui/react';
import { customColors } from 'core/colors';
import type { TransitionStatus } from 'react-transition-group';

const bgColorOverlay = {
  dark: 'rgba(191, 191, 191, 0.5)',
  light: 'rgba(171, 183, 183, 0.5)',
};

const bgColorPopup = {
  dark: 'gray.800',
  light: 'white',
};

type SecretPhraseConfirmationPopupProps = {
  duration: number;
  goNext: () => void;
  goPrev: () => void;
  isLoading: boolean;
  open: boolean;
  state: TransitionStatus;
};

function Logo() {
  return (
    <svg width="75" height="75" viewBox="0 0 75 75" fill="none" xmlns="http://www.w3.org/2000/svg">
      <circle cx="37.5" cy="37.5" r="37.5" fill="#00BFA5" fillOpacity="0.1" />
      <g clipPath="url(#clip0_2805_627602)">
        <path d="M24.305 22.7101L38 19.6667L51.695 22.7101C52.0651 22.7924 52.3961 22.9984 52.6334 23.2941C52.8706 23.5898 52.9999 23.9576 53 24.3367V40.9817C52.9999 42.628 52.5933 44.2487 51.8165 45.7001C51.0396 47.1515 49.9164 48.3886 48.5467 49.3017L38 56.3334L27.4533 49.3017C26.0838 48.3888 24.9608 47.1519 24.1839 45.7008C23.4071 44.2498 23.0004 42.6294 23 40.9834V24.3367C23.0001 23.9576 23.1294 23.5898 23.3666 23.2941C23.6039 22.9984 23.9349 22.7924 24.305 22.7101Z" fill="#12838E" />
      </g>
      <defs>
        <clipPath id="clip0_2805_627602">
          <rect width="40" height="40" fill="white" transform="translate(18 18)" />
        </clipPath>
      </defs>
    </svg>
  );
}

type BackdropProps = {
  children: JSX.Element;
  duration: number;
  state: TransitionStatus;
};

const SecretPhraseConfirmationBackdrop = forwardRef(
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

function SecretPhraseConfirmationPopup({
  duration, goNext, goPrev, isLoading, open, state,
}: SecretPhraseConfirmationPopupProps) {
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
    <SecretPhraseConfirmationBackdrop
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
          <Center mx="auto" mb={4} display="flex" flexDirection="column" height="100%" width="260px" justifyContent="center" flex="1">
            <Box pb={4} width="100%" display="flex" flexDirection="column" justifyContent="center" textAlign="center">
              <Center pb={4}>
                <Logo />
              </Center>
              <Text
                fontSize={23}
                fontWeight="bold"
              >
                Keep your phrase safe!
              </Text>
            </Box>
            <Box width="100%" display="flex" textAlign="center">
              <Text
                fontSize={16}
              >
                If you lose it you&apos;ll have no way of accessing your assets.
              </Text>
            </Box>
          </Center>
          <Box width="100%" display="flex" flexDirection="column" gap={3}>
            <Button
              width="100%"
              onClick={goPrev}
              height="48px"
              bgColor={bgColorPopup[colorMode]}
              border="1px"
              borderColor={customColors.navy[300]}
            >
              Show phrase again
            </Button>
            <Button
              width="100%"
              isLoading={isLoading}
              height="48px"
              colorScheme="salmon"
              color="white"
              onClick={goNext}
            >
              Done
            </Button>
          </Box>
        </>
        )}
      </Box>
    </SecretPhraseConfirmationBackdrop>
  );
}

export default SecretPhraseConfirmationPopup;
