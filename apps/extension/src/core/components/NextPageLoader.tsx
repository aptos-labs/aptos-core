// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { UseInfiniteQueryResult } from 'react-query';
import { useInView } from 'react-intersection-observer';
import React, { useEffect, useState } from 'react';
import { Box } from '@chakra-ui/react';

export interface NextPageLoaderProps<TData> {
  query: UseInfiniteQueryResult<TData>;
}

export default function NextPageLoader<TData>({ query }: NextPageLoaderProps<TData>) {
  const { inView, ref } = useInView();

  const [isFetchPending, setIsFetchPending] = useState<boolean>(inView);
  useEffect(() => {
    if (inView && query.hasNextPage && !isFetchPending) {
      setIsFetchPending(true);
      query.fetchNextPage()
        .then(() => setIsFetchPending(false));
    }
  }, [inView, query, isFetchPending]);

  return <Box ref={ref} />;
}
