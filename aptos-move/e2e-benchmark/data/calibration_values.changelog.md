# Move e2e-benchmark calibration log

Recalibration history, newest first. Each entry lists the tests whose calibrated value drifted out of band, as a signed `wall-time % change` (positive means slower); new tests show `new`.

## 2026-11-04

| transaction_type | runs | wall-time % change |
| --- | --- | --- |
| VectorTrimAppend { vec_len: 3000, element_len: 1, index: 0, repeats: 0 } | 15 | -17.0% |
| VectorTrimAppend { vec_len: 3000, element_len: 1, index: 2990, repeats: 1000 } | 15 | -19.6% |
| VectorRemoveInsert { vec_len: 3000, element_len: 1, index: 2998, repeats: 1000 } | 15 | -18.0% |
| VectorTrimAppend { vec_len: 100, element_len: 100, index: 0, repeats: 0 } | 15 | -17.2% |
| VectorTrimAppend { vec_len: 100, element_len: 100, index: 10, repeats: 1000 } | 15 | -21.8% |
| VectorRangeMove { vec_len: 100, element_len: 100, index: 50, move_len: 10, repeats: 1000 } | 15 | -31.1% |

## 2026-08-27

| transaction_type | runs | wall-time % change |
| --- | --- | --- |
| VectorPicture { length: 30720 } | 14 | +25.0% |
| VectorPictureRead { length: 30720 } | 14 | +26.2% |

