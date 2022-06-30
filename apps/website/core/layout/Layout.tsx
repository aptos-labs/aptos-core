import { Box } from '@chakra-ui/react';
import Footer from 'core/components/Footer';
import Header from 'core/components/Header';

interface LayoutProps {
  children: React.ReactNode;
}

const Layout = ({
  children,
}: LayoutProps) => (
  <Box width="100%">
    <Header />
    {children}
    <Footer />
  </Box>
);

export default Layout;
