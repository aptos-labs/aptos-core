# Move e2e-benchmark calibration log

Recalibration history, newest first. Each entry lists the tests whose calibrated value drifted out of band, as a signed `wall-time % change` (positive means slower); new tests show `new`.

## 2026-08-27

| transaction_type | runs | wall-time % change |
| --- | --- | --- |
| VectorPicture { length: 30720 } | 14 | +25.0% |
| VectorPictureRead { length: 30720 } | 14 | +26.2% |

