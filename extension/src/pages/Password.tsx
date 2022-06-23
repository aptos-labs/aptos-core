import {
  Box,
  Center,
  Heading,
  HStack,
  PinInput, PinInputField, useColorMode, VStack,
} from '@chakra-ui/react';
import { AptosBlackLogo, AptosWhiteLogo } from 'core/components/AptosLogo';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import { secondaryBgColor } from 'core/constants';
import React, { useCallback, useState } from 'react';

function Password() {
  const { colorMode } = useColorMode();
  const [pin, setPin] = useState<string>('');

  const pinOnChange = useCallback((value: string) => {
    setPin(value);
    if (value.length === 6) {
      // eslint-disable-next-line no-alert
      alert(pin);
    }
  }, []);

  return (
    <VStack
      bgColor={secondaryBgColor[colorMode]}
      justifyContent="center"
      spacing={4}
      width="100%"
      height="100%"
    >
      <Center>
        <Box width="75px" pb={4}>
          {
            (colorMode === 'dark')
              ? <AptosWhiteLogo />
              : <AptosBlackLogo />
          }
        </Box>
      </Center>
      <VStack>
        <Heading size="md" fontWeight={500} pb={2}>
          Enter your passcode
        </Heading>
        <HStack>
          <PinInput type="alphanumeric" mask autoFocus onChange={pinOnChange}>
            <PinInputField />
            <PinInputField />
            <PinInputField />
            <PinInputField />
            <PinInputField />
            <PinInputField />
          </PinInput>
        </HStack>
      </VStack>
    </VStack>
  );
}

export default withSimulatedExtensionContainer({ Component: Password });
