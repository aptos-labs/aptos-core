const bigInt=require("big-integer");





class ZqBuilder {
    constructor(q, name) {
        this.q=bigInt(q);
        this.h = [];
        this.c = [];
        this.name = name;
    }

    build() {
        this._buildHeaders();
        this._buildAdd();
        this._buildMul();

        this.c.push(""); this.h.push("");
        return [this.h.join("\n"), this.c.join("\n")];
    }

    _buildHeaders() {
        this.n64 = Math.floor((this.q.bitLength() - 1) / 64)+1;
        this.h.push("typedef unsigned long long u64;");
        this.h.push(`typedef u64 ${this.name}Element[${this.n64}];`);
        this.h.push(`typedef u64 *P${this.name}Element;`);
        this.h.push(`extern ${this.name}Element ${this.name}_q;`);
        this.h.push(`#define ${this.name}_N64 ${this.n64}`);
        this.c.push(`#include "${this.name.toLowerCase()}.h"`);
        this._defineConstant(`${this.name}_q`, this.q);
        this.c.push(""); this.h.push("");
    }

    _defineConstant(n, v) {
        let S = `${this.name}Element ${n}={`;
        const mask = bigInt("FFFFFFFFFFFFFFFF", 16);
        for (let i=0; i<this.n64; i++) {
            if (i>0) S = S+",";
            let shex = v.shiftRight(i*64).and(mask).toString(16);
            while (shex <16) shex = "0" + shex;
            S = S + "0x" + shex + "ULL";
        }
        S += "};";
        this.c.push(S);
    }

    _buildAdd() {
        this.h.push(`void ${this.name}_add(P${this.name}Element r, P${this.name}Element a, P${this.name}Element b);`);
        this.c.push(`void ${this.name}_add(P${this.name}Element r, P${this.name}Element a, P${this.name}Element b) {`);
        this.c.push("    __asm__ __volatile__ (");
        for (let i=0; i<this.n64; i++) {
            this.c.push(`        "movq ${i*8}(%2), %%rax;"`);
            this.c.push(`        "${i==0 ? "addq" : "adcq"} ${i*8}(%1), %%rax;"`);
            this.c.push(`        "movq %%rax, ${i*8}(%0);"`);
        }
        this.c.push("        \"jc SQ;\"");
        for (let i=0; i<this.n64; i++) {
            if (i>0) {
                this.c.push(`        "movq ${(this.n64 - i-1)*8}(%0), %%rax;"`);
            }
            this.c.push(`        "cmp ${(this.n64 - i-1)*8}(%3), %%rax;"`);
            this.c.push("        \"jg SQ;\"");
            this.c.push("        \"jl DONE;\"");
        }
        this.c.push("        \"SQ:\"");
        for (let i=0; i<this.n64; i++) {
            this.c.push(`        "movq ${i*8}(%3), %%rax;"`);
            this.c.push(`        "${i==0 ? "subq" : "sbbq"} %%rax, ${i*8}(%0);"`);
        }
        this.c.push("        \"DONE:\"");
        this.c.push(`    :: "r" (r), "r" (a), "r" (b), "r" (${this.name}_q) : "%rax", "memory");`);
        this.c.push("}\n");
    }

    _buildMul() {

        let r0, r1, r2;
        function setR(step) {
            if ((step % 3) == 0) {
                r0 = "%%r8";
                r1 = "%%r9";
                r2 = "%%r10";
            } else if ((step % 3) == 1) {
                r0 = "%%r9";
                r1 = "%%r10";
                r2 = "%%r8";
            } else {
                r0 = "%%r10";
                r1 = "%%r8";
                r2 = "%%r9";
            }
        }
        const base = bigInt.one.shiftLeft(64);
        const np64 = base.minus(this.q.modInv(base));

        this.h.push(`void ${this.name}_mul(P${this.name}Element r, P${this.name}Element a, P${this.name}Element b);`);
        this.c.push(`void ${this.name}_mul(P${this.name}Element r, P${this.name}Element a, P${this.name}Element b) {`);
        this.c.push("    __asm__ __volatile__ (");

        this.c.push(`        "subq $${this.n64*8}, %%rsp;"`);
        this.c.push(`        "movq $0x${np64.toString(16)}, %%r11;"`);
        this.c.push("        \"movq $0x0, %%r8;\"");
        this.c.push("        \"movq $0x0, %%r9;\"");
        this.c.push("        \"movq $0x0, %%r10;\"");

        for (let i=0; i<this.n64*2; i++) {
            setR(i);

            for (let o1=Math.max(0, i-this.n64+1); (o1<=i)&&(o1<this.n64); o1++) {
                const o2= i-o1;
                this.c.push(`        "movq ${o1*8}(%1), %%rax;"`);
                this.c.push(`        "mulq ${o2*8}(%2);"`);
                this.c.push(`        "addq %%rax, ${r0};"`);
                this.c.push(`        "adcq %%rdx, ${r1};"`);
                this.c.push(`        "adcq $0x0, ${r2};"`);
            }

            for (let j=i-1; j>=0; j--) {
                if (((i-j)<this.n64)&&(j<this.n64)) {
                    this.c.push(`        "movq ${j*8}(%%rsp), %%rax;"`);
                    this.c.push(`        "mulq ${(i-j)*8}(%3);"`);
                    this.c.push(`        "addq %%rax, ${r0};"`);
                    this.c.push(`        "adcq %%rdx, ${r1};"`);
                    this.c.push(`        "adcq $0x0, ${r2};"`);
                }
            }

            if (i<this.n64) {
                this.c.push(`        "movq ${r0}, %%rax;"`);
                this.c.push("        \"mulq %%r11;\"");
                this.c.push(`        "movq %%rax, ${i*8}(%%rsp);"`);
                this.c.push("        \"mulq (%3);\"");
                this.c.push(`        "addq %%rax, ${r0};"`);
                this.c.push(`        "adcq %%rdx, ${r1};"`);
                this.c.push(`        "adcq $0x0, ${r2};"`);
            } else {
                this.c.push(`        "movq ${r0}, ${(i-this.n64)*8}(%0);"`);
                this.c.push(`        "movq $0, ${r0};"`);
            }
        }

        this.c.push(`        "cmp  $0, ${r1};"`);
        this.c.push("        \"jne SQ2;\"");
        for (let i=0; i<this.n64; i++) {
            this.c.push(`        "movq ${(this.n64 - i-1)*8}(%0), %%rax;"`);
            this.c.push(`        "cmp ${(this.n64 - i-1)*8}(%3), %%rax;"`);
            this.c.push("        \"jg SQ2;\"");
            this.c.push("        \"jl DONE2;\"");
        }
        this.c.push("        \"SQ2:\"");
        for (let i=0; i<this.n64; i++) {
            this.c.push(`        "movq ${i*8}(%3), %%rax;"`);
            this.c.push(`        "${i==0 ? "subq" : "sbbq"} %%rax, ${i*8}(%0);"`);
        }
        this.c.push("        \"DONE2:\"");
        this.c.push(`        "addq $${this.n64*8}, %%rsp;"`);

        this.c.push(`    :: "r" (r), "r" (a), "r" (b), "r" (${this.name}_q) : "%rax", "%rdx", "%r8", "%r9", "%r10", "%r11", "memory");`);
        this.c.push("}\n");
    }

    _buildIDiv() {
        this.h.push(`void ${this.name}_idiv(P${this.name}Element r, P${this.name}Element a, P${this.name}Element b);`);
        this.c.push(`void ${this.name}_idiv(P${this.name}Element r, P${this.name}Element a, P${this.name}Element b) {`);
        this.c.push("    __asm__ __volatile__ (");
        this.c.push("        \"pxor %%xmm0, %%xmm0;\"");  // Comparison Register
        if (this.n64 == 1) {
            this.c.push(`        "mov %%rax, $${this.n64 - 8};"`);

        } else {
            this.c.push(`        "mov %%rax, $${this.n64 -16};"`);
        }

        this.c.push(`    :: "r" (r), "r" (a), "r" (b), "r" (${this.name}_q) : "%rax", "%rdx", "%r8", "%r9", "%r10", "%r11", "memory");`);
        this.c.push("}\n");
    }
}

var runningAsScript = !module.parent;

if (runningAsScript) {
    const fs = require("fs");
    var argv = require("yargs")
        .usage("Usage: $0 -q [primeNum] -n [name] -oc [out .c file] -oh [out .h file]")
        .demandOption(["q","n"])
        .alias("q", "prime")
        .alias("n", "name")
        .argv;

    const q = bigInt(argv.q);

    const cFileName =  (argv.oc) ? argv.oc : argv.name.toLowerCase() + ".c";
    const hFileName =  (argv.oh) ? argv.oh : argv.name.toLowerCase() + ".h";

    const builder = new ZqBuilder(q, argv.name);

    const res = builder.build();

    fs.writeFileSync(hFileName, res[0], "utf8");
    fs.writeFileSync(cFileName, res[1], "utf8");
} else {
    module.exports = function(q, name) {
        const builder = new ZqBuilder(q, name);
        return builder.build();
    };
}

