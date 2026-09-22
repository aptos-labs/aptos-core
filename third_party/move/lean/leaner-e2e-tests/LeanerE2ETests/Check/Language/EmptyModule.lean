-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

-- Port of v0 Language/EmptyModule: a module remains valid after all inline
-- declarations have been removed by a backend projection.
leaner module 0x42::empty_module where

#leaner_unit 0x42::empty_module

example : «0x42».empty_module.unit.borrowDiagnostics.isEmpty = true := by decide
