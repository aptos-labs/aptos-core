import HomeBody from 'core/components/HomeBody';
import {
  BASE_URL,
  COMPANY_NAME, COMPANY_URL, DEFAULT_SEO_DESCRIPTION,
} from 'core/constants';
import Layout from 'core/layout/Layout';
import type { NextPage } from 'next';
import { NextSeo } from 'next-seo';

const image = 'Petra_Card.png';
const imageUrl = `${BASE_URL}/${image}` as const;

const Home: NextPage = () => (
  <Layout>
    <NextSeo
      title={COMPANY_NAME}
      description={DEFAULT_SEO_DESCRIPTION}
      openGraph={{
        description: DEFAULT_SEO_DESCRIPTION,
        images: [
          {
            alt: COMPANY_NAME,
            height: 600,
            type: 'image/jpeg',
            url: imageUrl,
            width: 800,
          },
        ],
        site_name: COMPANY_NAME,
        title: COMPANY_NAME,
        url: COMPANY_URL,
      }}
    />
    <HomeBody />
  </Layout>
);

export default Home;
