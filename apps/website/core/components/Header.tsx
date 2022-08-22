import { MoonIcon, SunIcon } from '@chakra-ui/icons';
import {
  Button, Flex, Grid, HStack, IconButton, useColorMode,
} from '@chakra-ui/react';
import { FaDiscord } from 'react-icons/fa';
import { useMemo } from 'react';
import { COMPANY_NAME } from 'core/constants';
import ChakraLink, { ChakraLinkBare } from './ChakraLink';

const Header = () => {
  const { colorMode, setColorMode } = useColorMode();

  const colorModeIcon = useMemo(() => ((colorMode === 'light') ? <MoonIcon /> : <SunIcon />), [colorMode]);
  const oppositeColor = useMemo(() => ((colorMode === 'dark') ? 'light' : 'dark'), [colorMode]);

  return (
    <Grid
      width="100%"
      height="calc(40px + 32px)"
      py={4}
      templateColumns="107px 1fr"
      px={4}
    >
      <Flex alignItems="center">
        <ChakraLink href="/" fontSize="lg" fontWeight={600} verticalAlign="middle">
          {COMPANY_NAME}
        </ChakraLink>
      </Flex>
      <HStack justifyContent="flex-end" spacing={[2, 4, 4]}>
        <ChakraLinkBare href="/docs" passHref>
          <Button variant="ghost" as="a">
            Docs
          </Button>
        </ChakraLinkBare>
        <ChakraLinkBare
          href="https://discord.com/invite/petrawallet"
        >
          <IconButton
            as="a"
            target="_blank"
            variant="ghost"
            icon={<FaDiscord />}
            aria-label="Discord"
          />
        </ChakraLinkBare>
        <IconButton
          icon={colorModeIcon}
          onClick={() => setColorMode(oppositeColor)}
          aria-label="Color mode selector"
        />
      </HStack>
    </Grid>
  );
};
export default Header;
