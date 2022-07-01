import '../styles/globals.css';
import type { AppProps } from 'next/app';
import {
  ChakraProvider,
  extendTheme,
  ThemeConfig,
} from '@chakra-ui/react';
import { MDXProvider } from '@mdx-js/react';
import MDXComponents from 'core/components/MDXComponents';

const theme: ThemeConfig = extendTheme({
  initialColorMode: 'light',
  styles: {
    global: {
      'html, body': {
        margin: 0,
        padding: 0,
      },
    },
  },
  useSystemColorMode: false,
});

function MyApp({ Component, pageProps }: AppProps) {
  return (
    <ChakraProvider theme={theme}>
      <MDXProvider components={MDXComponents}>
        <Component {...pageProps} />
      </MDXProvider>
    </ChakraProvider>
  );
}

export default MyApp;
