-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- A module remains valid after all inline
-- declarations have been removed by a backend projection.
verex module 0x42::empty_module where

#verex_unit 0x42::empty_module

example : «0x42».empty_module.unit.borrowRejections.isEmpty = true := by decide
