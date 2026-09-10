-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-- Literal data and constructor equality normalization at native contract
boundaries, shared by target, hypothesis, and relational normalization. -/
register_simp_attr leaner_native_data_norm
