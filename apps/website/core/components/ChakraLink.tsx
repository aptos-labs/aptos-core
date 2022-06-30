import Link from 'next/link';
import { chakra, Text } from '@chakra-ui/react';

const ChakraLinkFactoryComponent = chakra(Link);

type ChakraLinkType = typeof ChakraLinkFactoryComponent;

const ChakraLink: ChakraLinkType = ({
  children,
  href,
  ...rest
}) => (
  <ChakraLinkFactoryComponent href={href} passHref>
    <Text as="a" {...rest}>
      {children}
    </Text>
  </ChakraLinkFactoryComponent>
);
export default ChakraLink;
