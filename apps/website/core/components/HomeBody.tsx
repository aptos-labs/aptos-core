import {
  Button,
  Center,
  HStack,
  Text, VStack,
} from '@chakra-ui/react';
import { COMPANY_NAME } from 'core/constants';
import LoginDemo from './LoginDemo';

const HomeBody = () => (
  <VStack pb={24}>
    <VStack pt={16} px={4} pb={8}>
      <Text fontSize={['2xl', '3xl', '3xl']}>
        {COMPANY_NAME}
      </Text>
      <Text textAlign="center" fontWeight={600} fontSize={['4xl', '5xl', '6xl']} marginTop="0px !important">
        Your tool to explore the world
      </Text>
      <Center>
        <HStack>
          <Button size="md" fontWeight={400} colorScheme="blue" borderRadius="full">
            Learn more
          </Button>
        </HStack>
      </Center>
    </VStack>
    <VStack pt={16} pb={8}>
      <LoginDemo />
    </VStack>
  </VStack>
);

export default HomeBody;
