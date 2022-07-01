import {
  Heading,
  Text,
  useColorMode,
} from '@chakra-ui/react';
import { secondaryLinkColor } from 'core/colors';
import { COMPANY_NAME } from 'core/constants';
import dynamic from 'next/dynamic';
import ChakraLink from './ChakraLink';

const ifmH2VerticalRhythmTop = 2;
const ifmH3VerticalRhythmTop = 1.5;
const ifmH1VerticalRhythmBottom = 1.25;
const ifmHeadingVerticalRhythmBottom = 1;
const ifmH3FontSize = '1.5rem';
const ifmH2FontSize = '2rem';
const ifmH1FontSize = '3rem';
const ifmListLeftPadding = '2rem';
const ifmListMargin = '1rem';
const ifmListItemMargin = '.25rem';

const ifmLeadingDesktop = 1.25;

const ifmLeading = `calc(${ifmLeadingDesktop} * 1rem)`;

const DynamicCodeBlock = dynamic(() => import('./CodeBlock'), {
  suspense: true,
});

const CompanyName = () => <span>{COMPANY_NAME}</span>;

const Heading1 = (props: any) => {
  const marginBottom = `calc(${ifmH1VerticalRhythmBottom} * ${ifmLeading})`;
  return (
    <Heading
      marginTop={0}
      marginBottom={marginBottom}
      as="h1"
      wordBreak="break-word"
      fontSize={ifmH1FontSize}
      {...props}
    />
  );
};

const Heading2 = (props: any) => {
  const marginTop = `calc(${ifmH2VerticalRhythmTop} * ${ifmLeading})`;
  const marginBottom = `calc(${ifmHeadingVerticalRhythmBottom} * ${ifmLeading})`;
  return (
    <Heading
      as="h2"
      marginBottom={marginBottom}
      {...props}
      marginTop={marginTop}
      wordBreak="break-word"
      fontSize={ifmH2FontSize}

    />
  );
};

const Heading3 = (props: any) => {
  const marginTop = `calc(${ifmH3VerticalRhythmTop} * ${ifmLeading})`;
  const marginBottom = `calc(${ifmHeadingVerticalRhythmBottom} * ${ifmLeading})`;
  return (
    <Heading
      as="h3"
      marginBottom={marginBottom}
      {...props}
      marginTop={marginTop}
      wordBreak="break-word"
      fontSize={ifmH3FontSize}
    />
  );
};

const Paragraph = (props: any) => {
  const marginBottom = `${ifmLeading}`;
  return (
    <Text
      as="p"
      marginBottom={marginBottom}
      {...props}
    />
  );
};

const listMargin = `0 0 ${ifmListMargin}`;
const listPaddingLeft = ifmListLeftPadding;

const OrderedList = (props: any) => (
  <ol
    style={{
      margin: listMargin,
      paddingLeft: listPaddingLeft,
    }}
    {...props}
  />
);

const UnorderedList = (props: any) => (
  <ul
    style={{
      margin: listMargin,
      paddingLeft: listPaddingLeft,
    }}
    {...props}
  />
);

const Link = ({
  href,
  ...rest
}: any) => {
  const hrefIsLocal = href.toString().startsWith('/');
  const { colorMode } = useColorMode();
  return (
    <ChakraLink
      _hover={{ textDecoration: 'underline' }}
      color={secondaryLinkColor[colorMode]}
      target={(!hrefIsLocal) ? '_blank' : undefined}
      href={href}
      {...rest}
    />
  );
};

const ListItem = (props: any) => (
  <li
    style={{
      marginTop: ifmListItemMargin,
    }}
    {...props}
  />
);

export const components = {
  CompanyName,
  a: Link,
  code: DynamicCodeBlock,
  h1: Heading1,
  h2: Heading2,
  h3: Heading3,
  li: ListItem,
  ol: OrderedList,
  p: Paragraph,
  ul: UnorderedList,
};

export default components;
