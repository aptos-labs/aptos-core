import React, { SyntheticEvent } from 'react';
import { Box, Tooltip, useClipboard } from '@chakra-ui/react';

interface CopyableProps {
  children: React.ReactNode | React.ReactNode[],
  copiedPrompt?: string,
  prompt?: string,
  timeout?: number,
  value: any,
}

/**
 * Copy the specified `value` when children are clicked, and give feedback with a tooltip
 * @constructor
 */
export default function Copyable({
  children,
  value,
  ...options
}: CopyableProps) {
  const prompt = options.prompt ?? 'Copy';
  const copiedPrompt = options.copiedPrompt ?? 'Copied!';
  const timeout = options.timeout ?? 500;

  const { hasCopied, onCopy } = useClipboard(value, { timeout });

  // Callback wrapper to prevent bubbling up
  const onClick = (e: SyntheticEvent) => {
    e.preventDefault();
    onCopy();
  };

  return (
    <Box cursor="pointer" onClick={onClick} as="span">
      <Tooltip
        label={hasCopied ? copiedPrompt : prompt}
        isOpen={hasCopied || undefined}
      >
        { children }
      </Tooltip>
    </Box>
  );
}
