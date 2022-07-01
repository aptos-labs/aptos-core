import { useMemo } from 'react';
import {
  Box,
  Button,
  Grid,
  Select,
  SimpleGrid,
  Text,
  useColorMode,
  useMediaQuery,
  VStack,
} from '@chakra-ui/react';
import { ChakraLinkBare } from 'core/components/ChakraLink';
import Footer from 'core/components/Footer';
import Header from 'core/components/Header';
import type { DocsGetStaticPropsReturn } from 'pages/docs/[slug]';
import { ChevronLeftIcon, ChevronRightIcon } from '@chakra-ui/icons';
import { secondaryMDXNavigationBorderColor, secondaryMDXNavigationFontColor } from 'core/colors';
import { useRouter } from 'next/router';

interface MDXItemProps {
  docsPath: string;
  pathObject: DocsGetStaticPropsReturn['props']['paths'][0]
}

const MDXItem = ({
  docsPath,
  pathObject,
}: MDXItemProps) => {
  const isCurrentPage = docsPath?.includes(pathObject.slug);
  return (
    <ChakraLinkBare href={pathObject.path}>
      <Button
        py={0}
        as="a"
        variant="ghost"
        fontWeight={isCurrentPage ? 600 : 400}
      >
        <span style={{
          maxWidth: 'calc(300px - 32px)',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
        >
          {pathObject.data?.title || pathObject.slug}

        </span>
      </Button>
    </ChakraLinkBare>
  );
};

interface DocsLayoutProps {
  children: React.ReactNode;
  docsPath: string;
  paths: DocsGetStaticPropsReturn['props']['paths']
}

const DocsLayout = ({
  children,
  docsPath,
  paths,
}: DocsLayoutProps) => {
  const { colorMode } = useColorMode();
  const router = useRouter();
  const [isLargerThan992] = useMediaQuery('(min-width: 992px)');
  const currPathObject = useMemo(
    () => paths.find((item) => item.path === docsPath),
    [docsPath, paths],
  );
  const nextPagePath: string | null = currPathObject?.data.nextPagePath;
  const nextPageTitle: string | null = currPathObject?.data.nextPageTitle;

  const prevPagePath: string | null = currPathObject?.data.prevPagePath;
  const prevPageTitle: string | null = currPathObject?.data.prevPageTitle;

  const selectOnChange = (path: string) => {
    router.push(path, undefined);
  };

  return (
    <Box width="100%">
      <Header />
      <Grid templateColumns={['1fr', '1fr', '1fr', '300px 1fr', '300px 1fr 200px']} maxW="100vw">
        <VStack padding={0} alignItems="flex-start" spacing={1}>
          {
            isLargerThan992 ? (
              paths.map((pathObject) => (
                <MDXItem
                  key={pathObject.path}
                  docsPath={docsPath}
                  pathObject={pathObject}
                />
              ))
            ) : (
              <Box width="100%" px={4} pb={2}>
                <Select
                  value={docsPath}
                  onChange={(event) => selectOnChange(event.target.value)}
                >
                  {
                  paths.map((pathObject) => (
                    <option
                      key={pathObject.slug}
                      value={pathObject.path}
                    >
                      {pathObject.data.title}
                    </option>
                  ))
                }
                </Select>
              </Box>
            )
          }
        </VStack>
        <Box px={4} pb={16} maxW="calc(100vw)">
          {children}
          <SimpleGrid columns={2} pt={16} gap={4}>
            {
              prevPagePath ? (
                <ChakraLinkBare href={prevPagePath}>
                  <VStack
                    as="a"
                    alignItems="flex-start"
                    padding={4}
                    border="1px solid"
                    borderColor={secondaryMDXNavigationBorderColor[colorMode]}
                    borderRadius=".5rem"
                    _hover={{
                      borderColor: secondaryMDXNavigationFontColor[colorMode],
                    }}
                  >
                    <Text pl={1}>
                      Previous
                    </Text>
                    <Text
                      fontWeight={600}
                      color={secondaryMDXNavigationFontColor[colorMode]}
                      textAlign="left"
                    >
                      <ChevronLeftIcon />
                      {prevPageTitle}
                    </Text>
                  </VStack>
                </ChakraLinkBare>
              ) : <Box />
            }
            {
              nextPagePath ? (
                <ChakraLinkBare href={nextPagePath}>
                  <VStack
                    as="a"
                    alignItems="flex-end"
                    padding={4}
                    border="1px solid"
                    borderColor={secondaryMDXNavigationBorderColor[colorMode]}
                    borderRadius=".5rem"
                    _hover={{
                      borderColor: secondaryMDXNavigationFontColor[colorMode],
                    }}
                  >
                    <Text pr={1}>
                      Next
                    </Text>
                    <Text
                      fontWeight={600}
                      color={secondaryMDXNavigationFontColor[colorMode]}
                      textAlign="right"
                    >
                      {nextPageTitle}
                      <ChevronRightIcon />
                    </Text>
                  </VStack>
                </ChakraLinkBare>
              ) : <Box />
            }
          </SimpleGrid>
        </Box>
      </Grid>
      <Footer />
    </Box>
  );
};

export default DocsLayout;
