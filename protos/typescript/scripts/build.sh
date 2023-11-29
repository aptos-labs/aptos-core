# Build the commonjs code. We include a package.json with the type set to commonjs
# so the downstream code knows that it is commonjs and not esm.
tsc --module commonjs --outDir dist/cjs
echo '{"type": "commonjs"}' > dist/cjs/package.json

# Build the esm code. We include a package.json with the type set to esm
# so the downstream code knows that it is esm and not commonjs.
tsc --module es2022 --outDir dist/esm
echo '{"type": "module"}' > dist/esm/package.json
