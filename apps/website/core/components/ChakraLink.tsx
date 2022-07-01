import Link from 'next/link';
import { chakra, Text } from '@chakra-ui/react';

export const ChakraLinkFactoryComponent = chakra(Link);

export type ChakraLinkType = typeof ChakraLinkFactoryComponent;

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

/**
 * @description To be used with a custom anchor tag
 * @example
 * <ChakraLinkBare href="https://google.com">
 *  <Button as="a" variant="ghost" />
 * </ChakraLinkBare>
 */
export const ChakraLinkBare: ChakraLinkType = ({
  children,
  href,
}) => (
  <ChakraLinkFactoryComponent href={href} passHref>
    {children}
  </ChakraLinkFactoryComponent>
);

export default ChakraLink;
