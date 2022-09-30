// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Drawer, DrawerOverlay, DrawerContent,
} from '@chakra-ui/react';
import {
  TransferDrawerPage, useTransferFlow,
} from 'core/hooks/useTransferFlow';
import React, { useMemo } from 'react';
import { customColors } from 'core/colors';
import { transparentize } from 'color2k';
import TransferDrawerConfirm from './TransferDrawerConfirm';
import TransferDrawerAmount from './TransferDrawerAmount';

function TransferDrawerSwitch() {
  const { transferDrawerPage } = useTransferFlow();

  const drawerSwitch = useMemo(() => {
    switch (transferDrawerPage) {
      case TransferDrawerPage.ADD_ADDRESS_AND_AMOUNT:
        return <TransferDrawerAmount />;
      case TransferDrawerPage.CONFIRM_TRANSACTION:
        return <TransferDrawerConfirm />;
      default:
        return <TransferDrawerAmount />;
    }
  }, [transferDrawerPage]);

  return drawerSwitch;
}

export default function TransferDrawer() {
  const {
    closeDrawer, isDrawerOpen,
  } = useTransferFlow();
  return (
    <Drawer
      size="xl"
      isOpen={isDrawerOpen}
      onClose={closeDrawer}
      placement="bottom"
    >
      <DrawerOverlay bgColor={transparentize(customColors.navy[900], 0.5)} backdropFilter="blur(1rem)" />
      <DrawerContent className="drawer-content" borderTopRadius=".5rem">
        <TransferDrawerSwitch />
      </DrawerContent>
    </Drawer>
  );
}
