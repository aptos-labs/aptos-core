// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { Routes } from 'core/routes';
import { useNavigate } from 'react-router-dom';
import { loadBackgroundState } from 'core/utils/account';

function LoadState() {
  const navigate = useNavigate();

  useEffect(() => {
    loadBackgroundState().then(() => {
      navigate(Routes.wallet.routePath);
    });
  }, [navigate]);
  return null;
}

export default LoadState;
