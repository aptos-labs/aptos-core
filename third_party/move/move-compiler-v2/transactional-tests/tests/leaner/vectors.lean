-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerVectors where
  fun length() -> u64 := vector<u64>[10, 20, 30].length

  fun middle() -> u64 := do
    let values := vector<u64>[10, 20, 30]
    let value := &values[1]
    *value

  fun replaced() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let slot := &mut values[1]
    *slot := 42
    let value := &values[1]
    *value

  fun borrowed_mut() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    let value := &mut values[1]
    *value := 42
    *value

  fun out_of_range() -> u64 := do
    let values := vector<u64>[10, 20, 30]
    let value := &values[3]
    *value

--# run 0x0::LeanerVectors::length

--# run 0x0::LeanerVectors::middle

--# run 0x0::LeanerVectors::replaced

--# run 0x0::LeanerVectors::borrowed_mut

--# run 0x0::LeanerVectors::out_of_range
