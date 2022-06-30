import '../styles/globals.css';
import type { AppProps } from 'next/app';
import { ChakraProvider, extendTheme, ThemeConfig } from '@chakra-ui/react';

const theme: ThemeConfig = extendTheme({
  initialColorMode: 'light',
  styles: {
    global: {
      'html, body': {
        margin: 0,
        overflow: (process.env.NODE_ENV !== 'development') ? 'hidden' : undefined,
        padding: 0,
      },
    },
  },
  useSystemColorMode: false,
});

function MyApp({ Component, pageProps }: AppProps) {
  return (
    <ChakraProvider theme={theme}>
      <Component {...pageProps} />
    </ChakraProvider>
  );
}

export default MyApp;
