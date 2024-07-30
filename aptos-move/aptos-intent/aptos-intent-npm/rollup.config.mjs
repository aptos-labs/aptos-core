import { wasm } from '@rollup/plugin-wasm';
export default {
	input: 'entry.js',
	output: [
		{
			dir: 'dist/esm',
			format: "esm",
			
		},
		{
			dir: 'dist/cjs',
			format: "cjs"
		}
	],
	plugins: [wasm({
		maxFileSize: 10000000
	})]
};