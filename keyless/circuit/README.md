# Aptos Keyless circuit

## Installing dependencies

The scripts in this repository will not work without installing the dependencies.

To install, please run:

```
. ./tools/install-deps.sh
```

## Run sub-circuit unittests

```bash
cargo test -p aptos-keyless-circuit
```

## Generating the proving key

To generate a sample prover and verifier key pair, run the following commands:

```
./tools/trusted-setup.sh sample_keypair
```

## Testing

TODO: update `input_gen.py` to match the latest circuit, then provide instructions.

## Generating a sample proof

TODO: update `create-proofs-for-testing.sh` to match the latest circuit, then provide instructions.

## Circuit stats

Command:
```
circom -l `npm root -g` templates/main.circom --r1cs
```

Output:
```
non-linear constraints: 1376867
linear constraints: 0
public inputs: 1
private inputs: 7858 (7745 belong to witness)
public outputs: 0
wires: 1343588
labels: 6286968
```
