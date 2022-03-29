# fgi

`fgi` is the entrypoint to the Forge unified testing framework. It is a python script with minimal dependencies outside of the python standard library, for portability. `fgi` must be run from the Aptos project root.

To run, you must have `kubectl` and `helm` installed, and have the proper permissions to access the clusters specified in `kube.py`.

```
# fgi must be run from the Aptos project root
./scripts/fgi/run -h
```
