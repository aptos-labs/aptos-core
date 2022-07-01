import { Code } from '@chakra-ui/react';
import SyntaxHighlighter from 'react-syntax-highlighter/dist/cjs/prism';
import { a11yDark } from 'react-syntax-highlighter/dist/cjs/styles/prism';

interface CodeBlockProps {
  [x: string]: any
}

function CodeBlock({ children, className, ...props }: CodeBlockProps) {
  const match = /language-(\w+)/.exec(className || '');
  return match
    ? (
      <SyntaxHighlighter
        language={match[1]}
        style={a11yDark}
        PreTag="div"
        wrapLongLines
        {...props}
      >
        {children}
      </SyntaxHighlighter>
    )
    : (
      <Code
        className={className}
        {...props}
      >
        {children}
      </Code>
    );
}

export default CodeBlock;
