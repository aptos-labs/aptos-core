const fs = require("fs");
const fetch = require('node-fetch');

const input = fs.readFileSync(process.argv[2], "utf8");
const circuit = process.argv[3];

async function callProve() {
    const rawResponse = await fetch(`http://localhost:8080/prove`, {
      method: 'POST',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json'
      },
      body: input
    });
    if (rawResponse.ok) {
        return rawResponse.json();
    } else {
        throw new Error(rawResponse.status);
    }
};

async function run() {
    let j = await callProve();
    console.log(JSON.stringify(j, null,1));
}

run().then(() => {
    process.exit(0);
}, (err) => {
    console.log("ERROR");
    console.log(err);
    process.exit(1);
});
