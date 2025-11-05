# Benchmarks

## Batched Bulletproofs and DeKART

Go to `aptos-crypto`:
```
cd crates/aptos-crypto
```

Install [`criterion-means`](https://crates.io/crates/cargo-criterion-means):

```
cargo install criterion-means
```

Run the Bulletproof benchmarks:
```
RAYON_NUM_THREADS=1 cargo bench -- bulletproofs
```

Run the DeKART benchmarks (**TODO:** Move DeKART into `aptos-crypto/`):
```
cd ../aptos-dkg/
RAYON_NUM_THREADS=1 cargo bench -- dekart-rs/bls12-381
cd -
```

Generate a CSV file of the means running times:
```
cargo criterion-means ../../ >range_proofs.csv
```

Paste the CSV data into ChatGPT with the following prompt:

 > % PROMPT STARTS HERE; DO NOT REMOVE! %
 > Generate nicely-formatted Markdown tables from the following CSV data.
 > There should be one table for each $\ell$ value.
 > The column names should be Scheme, $n$, Proving time (ms), Verify time (ms), Total time (ms), Proof size (bytes)
 > The data in the CSV file should be parsed so as to correctly fill in these tables.
 > Note that the "proof size" column should be empty (there is no data to put there).
 > Note that the CSV file contains StdErr, but it should be ignored.
 > Note that the CSV file contains both proving times and verification times, but they are on separate lines
 > Note that the times for proving and verification are in nanoseconds in the CSV file, so they should be converted to milliseconds.
 > Note that the "Total time" column should just add the proving time and verification time columns.
 > Note that $\ell$ is the number of bits and $n$ is the batch size, and they are both indicated in the data in the CSV file under the Parameter column.
 > Style the table so that the cells are whitespace padded and aligned correctly.
 > Collapse the Bulletproof scheme name to "Bulletproofs".
 > Collapse the Dekart scheme name to "DeKART (BLS12-381)".
 > Make sure all numbers are rounded to two decimal points only.
 > Sort the table as follows:
 > The table's rows should alternate between Bulletproofs and DeKART.
 > Then, sort by increasing $n$.
 > Before each markdown table, add an H4 heading (####) titled "$\ell = <ell>$ numbers"
 > % PROMPT ENDS HERE: DO NOT REMOVE! %

All of this can be done in one line via:
```
./run-range-proof-benches.sh
```
...which will copy both the CSV data and the ChatGPT prompt into your clipboard so you can just paste it in and get a nice Markdown formatted table.
