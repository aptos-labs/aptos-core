// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { getTokenIdStringFromDict, TokenId } from 'core/utils/token';
import React, { useMemo } from 'react';
import ChakraLink from './ChakraLink';
import SquareBox from './SquareBox';
import SquareImage from './SquareImage';

interface GalleryItemProps {
  id?: TokenId,
  imageSrc: string;
}

function GalleryItem({
  id,
  imageSrc,
}: GalleryItemProps) {
  const tokenIdString = useMemo(() => (id ? getTokenIdStringFromDict(id) : null), [id]);
  return (
    <ChakraLink to={`/tokens/${tokenIdString}`}>
      <SquareBox cursor="pointer">
        <SquareImage borderRadius=".5rem" src={imageSrc} />
      </SquareBox>
    </ChakraLink>
  );
}

export default GalleryItem;
