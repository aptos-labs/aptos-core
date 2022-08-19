import {
  Button,
  Center,
  HStack,
  Text, VStack,
} from '@chakra-ui/react';
import { COMPANY_NAME_WITH_WALLET } from 'core/constants';
import { ChakraLinkBare } from './ChakraLink';
import LoginDemo from './LoginDemo';

const HomeBody = () => (
  <VStack pb={24}>
    <VStack pt={16} px={4} pb={8}>
      <Text fontSize={['2xl', '3xl', '3xl']}>
        {COMPANY_NAME_WITH_WALLET}
      </Text>
      <Text textAlign="center" fontWeight={600} fontSize={['4xl', '5xl', '6xl']} marginTop="0px !important">
        Your tool to explore Aptos
      </Text>
      <Center>
        <HStack>
          <ChakraLinkBare href="/docs">
            <Button as="a" size="md" fontWeight={400} colorScheme="blue" borderRadius="full">
              Learn more
            </Button>
          </ChakraLinkBare>
        </HStack>
      </Center>
    </VStack>
    <VStack pt={16} pb={8}>
      <LoginDemo />
    </VStack>
  </VStack>
);

export default HomeBody;
