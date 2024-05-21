const wc  = require("./witness_calculator.js");
const { readFileSync, writeFile } = require("fs");

if (process.argv.length != 5) {
    console.log("Usage: node generate_witness.js <file.wasm> <input.json> <output.wtns>");
} else {
    const input = JSON.parse(readFileSync(process.argv[3], "utf8"));
    
    const buffer = readFileSync(process.argv[2]);
    wc(buffer).then(async witnessCalculator => {
	//    const w= await witnessCalculator.calculateWitness(input,0);
	//    for (let i=0; i< w.length; i++){
	//	console.log(w[i]);
	//    }
	const buff= await witnessCalculator.calculateWTNSBin(input,0);
	writeFile(process.argv[4], buff, function(err) {
	    if (err) throw err;
	});
    });
}
