import React from 'react';
import {
  Box,
  VStack,
} from '@chakra-ui/react';
import WalletLayout from 'core/layouts/WalletLayout';
import CredentialsBody from 'core/components/CredentialsBody';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';

function Credentials() {
  return (
    <WalletLayout backPage="/settings">
      <VStack width="100%" paddingTop={8}>
        <Box px={4} pb={4} width="100%">
          <CredentialsBody />
        </Box>
      </VStack>
    </WalletLayout>
  );
}

export default withSimulatedExtensionContainer(Credentials);
