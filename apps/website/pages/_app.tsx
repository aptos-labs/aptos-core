import '../styles/globals.css';
import type { AppProps } from 'next/app';
import {
  ChakraProvider,
  extendTheme,
  ThemeConfig,
} from '@chakra-ui/react';
import { MDXProvider } from '@mdx-js/react';
import MDXComponents from 'core/components/MDXComponents';
import { DefaultSeo } from 'next-seo';
import { COMPANY_NAME, COMPANY_URL } from 'core/constants';
import AnalyticsLayout from 'core/layout/AnalyticsLayout';

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
    <AnalyticsLayout>
      <ChakraProvider theme={theme}>
        <DefaultSeo
          openGraph={{
            locale: 'en_IE',
            site_name: COMPANY_NAME,
            type: 'website',
            url: COMPANY_URL,
          }}
          twitter={{
            cardType: 'summary_large_image',
            handle: '@PetraWallet',
            site: '@PetraWallet',
          }}
        />
        <MDXProvider components={MDXComponents}>
          <Component {...pageProps} />
        </MDXProvider>
      </ChakraProvider>
    </AnalyticsLayout>

  );
}

export default MyApp;
