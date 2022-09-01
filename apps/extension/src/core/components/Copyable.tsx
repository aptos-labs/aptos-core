import React from 'react';
import { Box, Tooltip, useClipboard } from '@chakra-ui/react';

export const defaultTimeout = 500;
export const defaultOpenDelay = 300;

interface CopyableProps {
  children: React.ReactNode | React.ReactNode[],
  copiedPrompt?: string,
  openDelay?: number,
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
  const timeout = options.timeout ?? defaultTimeout;
  const openDelay = options.openDelay ?? defaultOpenDelay;

  const { hasCopied, onCopy } = useClipboard(value, { timeout });

  // Callback wrapper to prevent bubbling up
  const onClick = (e: React.SyntheticEvent) => {
    e.preventDefault();
    e.stopPropagation();
    e.nativeEvent.stopImmediatePropagation();
    onCopy();
  };

  return (
    <Box cursor="pointer" onClick={onClick} as="span">
      <Tooltip
        label={hasCopied ? copiedPrompt : prompt}
        isOpen={hasCopied || undefined}
        openDelay={openDelay}
      >
        { children }
      </Tooltip>
    </Box>
  );
}
