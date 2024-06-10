# Aptos Keyless circuit

## Installing dependencies

The scripts in this repository will not work without installing the dependencies.

To install, please run:

```
./tools/install-deps.sh
```
## Generating the proving key

To generate a sample prover and verifier key pair, run the following commands:

```
./tools/trusted-setup.sh sample_keypair
```

## Testing
```commandline
python input_gen.py
cd templates
circom main.circom --wasm -l .
node main_js/generate_witness.js main_js/main.wasm ../input.json witness.wtns
```

When it has finished running, there will be two files, corresponding to the prover `.zkey` and verifier `.zkey` key each, in the `sample_keypair` directory. 

## Generating a sample proof

To generate a sample proof for the statement encoded in `input-gen.py`, do the following.

First, make sure you have a proving key set up (see [above](#generating-the-proving-key)).

Second, call the following script, where:

 - `<keyless-circuit-branch>` is the branch of the `keyless-circuit` repo that contains the version of the circuit you want to use
 - `<proving_key_path>` is the path to the proving key.
 - `<output_dir>` is an optional output directory where the `proof.json` and `public.json` files will be created.

```
 ./create-proofs-for-testing.sh <keyless-circuit-branch> <proving_key_path> [<output_dir>]
```
For example, the command could be:
```
 ./create-proofs-for-testing.sh main ../aptos-keyless-trusted-setup-contributions/contributions/main_final.zkey
```
(**Note:** Here, we are assuming the `main_final.zkey` proving key is in the [aptos-keyless-trusted-setup-contributions](https://github.com/aptos-labs/aptos-keyless-trusted-setup-contributions) repo, stored in the parent directory.)

This command will create two files in the current working directory:
1. An `input.json` file containing the inputs to the circuit that are to be proved
2. A `proof.json` file containing the actual proof.
3. A `public.json` file containing the public inputs under which the proof verifies

## Circuit stats

Command:
```
circom -l . main.circom --r1cs
```

Output:
```
non-linear constraints: 1299928
linear constraints: 0
public inputs: 1
private inputs: 7123 (7033 belong to witness)
public outputs: 0
wires: 1270049
labels: 6093448
```
