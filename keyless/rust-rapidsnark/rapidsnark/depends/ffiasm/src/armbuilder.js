const bigInt = require("big-integer");
const assert = require("assert");

module.exports.genFuncs = genFuncs

class Reg {
    constructor(number) {
        this.number = number;
    }
    valueOf()  { return this.number; }
    toString() {
        if (this.number === 31)
            return "xzr";
        else
            return `x${this.number}`;
    }
}

class RegVar extends Array {
    constructor(...args) {
        super(...args.map(x => new Reg(x)));
        this.cur = 0;
    }

    toString() {
        return this.join(", ");
    }

    getNext() {
        console.assert(this.length, "RegVar is empty, cannot get next Reg.");

        let reg = this[this.cur];
        this.cur = (this.cur + 1) % this.length;

        return reg;
    }

    rewind() {
        this.cur = 0;
    }
}

function removeSlice(regVar, start, number) {
    let piece = regVar.slice();

    piece.splice(start, number);
    return piece;
}

function cyclicCopy(regVar, width) {
    let regVarCopy = regVar.slice();

    let newVar = [];
    for(let i = 0; i < width; i++) {
        newVar.push(regVarCopy.getNext());
    }
    return newVar;
}

function expand(regVar, newReg, width)
{
    let newVar = regVar.slice();
    while(newVar.length < width) {
        newVar.push(new Reg(newReg));
    }
    return newVar;
}

class RegPool {
    constructor(numParamRegs) {
        this.numParamRegs = numParamRegs;
        this.m_maxAvailReg = 29;
        this.regPool = [];
        this.savedRegs = [];

        for (let reg = numParamRegs; reg <= this.m_maxAvailReg; reg++) {
            if (reg !== 18) {
                this.regPool.push(new Reg(reg));
            }
        }
    }

    assignRegs(numRegs) {
        let regs = new RegVar();

        for(let i = 0; i < numRegs; i++) {
            console.assert(this.regPool.length, "RegPool is empty, cannot assign Reg.");

            const reg = this.regPool.shift();
            regs.push(reg);

            if (reg >= 19) {
                this.savedRegs.push(reg);
            }
        }
        return regs;
    }

    releaseRegs(regs) {
        this.regPool.push(...regs);
    }

    hasSavedRegs() {
        return this.savedRegs.length > 0;
    }

    getSavedRegs() {
        return this.savedRegs.slice();
    }
}

class GenBase {
    constructor(width, space, name, numParamRegs, numWorkRegs) {
        this.code = [];
        this.Indent = "        ";
        this.width = width;
        this.space = space;
        this.name = name;
        this.regPool = new RegPool(numParamRegs);
        this.workRegs = this.regPool.assignRegs(numWorkRegs);

        assert(width > 0);

        this.op_func_name();
    }

    toString() {
        return this.getCode();
    }

    getCode() {
        return this.code.join("\n")+"\n";
    }

    add_line(...line) {
        this.code.push(line.join(""));
    }

    op(instrName, ...args) {
        let instr = this.Indent;

        if (args.length) {
            let operands = args.map(s => s.toString().padStart(3)).join(", ");
            instr += `${instrName.padEnd(5)} ${operands}`;
        } else {
            instr += `${instrName}`;
        }
        this.add_line(instr);
    }

    op_func_name() {
        this.add_line(`${this.space}_${this.name}:`);
        this.add_line(`_${this.space}_${this.name}:`);
    }

    op_label(label) {
        this.add_line(`${label}:`);
    }

    op_comment(...comment) {
        this.add_line(this.Indent + "// ", ...comment);
    }

    op_debug(...str) {
        this.op_comment(...str);
    }

    op_empty() {
        this.add_line("");
    }

    makeLabel(label_name) {
        return this.space + "_" + this.name + "_" + label_name;
    }

    getVarName(name) {
        return this.space + "_" + name;
    }

    getMem(regNum, offset) {
        if (offset === 0) {
            return "[x" + regNum + "]";
        }
        return "[x" + regNum + ", " + offset + "]";
    }

    getMemWord(regNum, wordNum) {
        return this.getMem(regNum, wordNum * 8);
    }

    hasSavedRegs() {
        return this.regPool.hasSavedRegs();
    }

    pushRegs() {
        let saved = this.regPool.getSavedRegs();
        let size = saved.length;

        for (let i = 0; i <size; i++) {
            this.genStoreWord(saved, i, "[sp, #-16]!");
        }

        if (size) this.op_empty();
    }

    popRegs() {
        let saved = this.regPool.getSavedRegs();
        let size = saved.length;

        if (size) this.op_empty();

        if (size % 2) {
            this.op("ldr", saved[size-1], "[sp], #16");
           size--;
        }

        for (let i =size; i > 0; i -= 2) {
            this.op("ldp", saved[i-2], saved[i-1], "[sp], #16");
        }
    }

    assignRegs(regs, numRegs) {
        return this.regPool.assignRegs(regs, numRegs);
    }

    releaseRegs(regs) {
        this.regPool.releaseRegs(regs);
    }

    getNextWorkReg() {
        return this.workRegs.getNext();
    }

    genStoreWord(regVar, i, memArg)
    {
        if (i % 2 == 0) {
            if (i === regVar.length - 1) {
                this.op("str", regVar[i], memArg);
            } else {
                this.op("stp", regVar[i], regVar[i+1], memArg);
            }
        }
    }

    genLoadWord(regVar, i, memArg)
    {
        if (i % 2 == 0) {
            if (i === regVar.length - 1) {
                this.op("ldr", regVar[i], memArg);
            } else {
                this.op("ldp", regVar[i], regVar[i+1], memArg);
            }
        }
    }

    genAddReg(r, a, b, i) {
        this.op(i ? "adcs" : "adds", r, a, b);
    }

    genAddWord(r, a, b, i) {
        this.genAddReg(r[i], a[i], b[i], i);
    }

    genAdd(r, a, b) {
        for (let i = 0; i < this.width; i++) {
            this.genAddWord(r, a, b, i);
        }
    }

    genSubReg(r, a, b, i) {
        this.op(i ? "sbcs" : "subs", r, a, b);
    }

    genSubWord(r, a, b, i) {
        this.genSubReg(r[i], a[i], b[i], i);
    }

    genSub(r, a, b) {
        for (let i = 0; i < this.width; i++) {
            this.genSubWord(r, a, b, i);
        }
    }

    genSubLoad(r, a, b, memReg) {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(b, i, memReg);
            this.genSubWord(r, a, b, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    genCselStore(r, a, b, memReg)
    {
        let j = 0;
        for (let i = 0; i < this.width; i++) {
            this.genCselWord(r, a, b, i);
            if (i > 0)
                this.genStoreVarWord(r, j++, memReg);
            if (i % 2) this.op_empty();
        }
        this.genStoreVarWord(r, j, memReg);
    }

    genCselWord(r, a, b, i) {
        this.op("csel", r[i], a[i], b[i], "hs");
    }

    genStoreVarWord(regVar, i, memReg) {
        this.genStoreWord(regVar, i, this.getMemWord(memReg, i));
    }

    genLoadVarWord(regVar, i, memReg) {
        this.genLoadWord(regVar, i, this.getMemWord(memReg, i));
    }

    genLoadVar(regVar, memReg) {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(regVar, i, memReg);
        }
    }

    genStoreVar(regVar, memReg) {
        for (let i = 0; i < this.width; i++) {
            this.genStoreVarWord(regVar, i, memReg);
        }
    }

    genLboMask(lastWord, maskReg) {
        this.op("adr", maskReg, this.getVarName("lboMask"));
        this.op("ldr", maskReg, this.getMem(maskReg, 0));
        this.op("and", lastWord, lastWord, maskReg);
        this.op_empty();
    }
}

class Gen_rawIsZero extends GenBase {
    constructor(width, space) {
        super(width, space, "rawIsZero", 1, 16)
        this.generate();
    }

    generate() {
        this.genFold();

        this.op("cmp", this.accum, "xzr");
        this.op("cset", "x0", "eq");
        this.op("ret");
    }

    genFold() {
        const chunkCount = Math.trunc(this.width / 4);
        const lastWidth  = this.width % 4;

        let i = 0;
        for (; i < chunkCount; i += 1) {
            this.genChunk(4, i);
        }

        if (lastWidth) {
            this.genChunk(lastWidth, i);
        }
    }

    genChunk(width, i) {
         let r = this.genStep(width, i * 4);

        if (this.accum === undefined) {
            this.accum = r;
        } else {
            this.op("orr", "x17", this.accum, r);

            this.accum = "x17";
        }

        this.op_empty();
    }

    genStep(width, i) {
        assert(width > 0);

        if (width === 1) {
            const r = this.getNextWorkReg();

            this.op("ldr", r, this.getMemWord(0, i));

            return r;

        } else if (width === 2) {
            const a = this.getNextWorkReg();
            const b = this.getNextWorkReg();
            const r = this.getNextWorkReg();

            this.op("ldp", a, b, this.getMemWord(0, i));
            this.op("orr", r, a, b);

            return r;

        } else {
            const a = this.genStep(2, i);
            this.op_empty();
            const b = this.genStep(width - 2, i + 2);
            const r = this.getNextWorkReg();

            this.op("orr", r, a, b);

            return r;
        }
    }
}

class Gen_rawIsEq extends GenBase {
    constructor(width, space) {
        super(width, space, "rawIsEq", 2, 3);

        this.a1 = cyclicCopy(this.assignRegs(4), width);
        this.b1 = cyclicCopy(this.assignRegs(4), width);
        this.c1 = cyclicCopy(this.assignRegs(4), width);

        this.generate();
    }

    generate() {
        if (this.longNumber()) {
            this.accum = "x17";

            this.op("mov", "x17", "xzr");
            this.op_empty();
        }

        this.genFold();

        this.op("cmp", this.accum, "xzr");
        this.op("cset", "x0", "eq");
        this.op("ret");
    }

    genFold() {
        const chunkCount = Math.trunc(this.width / 4);
        const lastWidth  = this.width % 4;

        let i = 0;
        for (; i < chunkCount; i += 1) {
            this.genChunk(4, i);
        }

        if (lastWidth) {
            this.genChunk(lastWidth, i);
        }
    }

    genChunk(width, i) {
        let r = this.genStep(width, i * 4);

        if (this.longNumber()) {
            this.op("orr", "x17", "x17", r);
        } else {
            if (this.accum === undefined) {
                this.accum = r;
            } else {
                this.op("orr", "x17", this.accum, r);

                this.accum = "x17";
            }
        }
        this.op_empty();
    }

   genStep(width, i) {
        assert(width > 0);

        if (width === 1) {
            const r = this.getNextWorkReg();

            this.op("ldr",    this.a1[i], this.getMemWord(0, i));
            this.op("ldr",    this.b1[i], this.getMemWord(1, i));
            this.op("eor", r, this.a1[i], this.b1[i]);

            return r;
        } else if (width === 2) {
            const r = this.getNextWorkReg();

            this.op("ldp", this.a1[i],   this.a1[i+1], this.getMemWord(0, i));
            this.op("ldp", this.b1[i],   this.b1[i+1], this.getMemWord(1, i));
            this.op("eor", this.c1[i],   this.a1[i],   this.b1[i]);
            this.op("eor", this.c1[i+1], this.a1[i+1], this.b1[i+1]);
            this.op("orr", r,         this.c1[i],   this.c1[i+1]);

            return r;
        } else {
            const a = this.genStep(2, i);
            this.op_empty();
            const b = this.genStep(width - 2, i + 2);
            const r = this.getNextWorkReg();

            this.op("orr", r, a, b);

            return r;
        }
    }

    longNumber() {
        return (this.width >= 8);
    }
}

class Gen_rawSwap extends GenBase {
    constructor(width, space) {
        super(width, space, "rawSwap", 2, 0);

        this.a1 = cyclicCopy(this.assignRegs(8), width);
        this.b1 = cyclicCopy(this.assignRegs(8), width);

        this.generate();
    }

    generate() {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.a1,  i, 0);
            this.genLoadVarWord(this.b1,  i, 1);
            this.genStoreVarWord(this.a1, i, 1);
            this.genStoreVarWord(this.b1, i, 0);
            if (i % 2) this.op_empty();
        }
        this.op("ret");
    }
}

class Gen_rawCopy extends GenBase {
    constructor(width, space) {
        super(width, space, "rawCopy", 2, 0);

        this.a1 = cyclicCopy(this.assignRegs(8), this.width);

        this.generate();
    }

    generate() {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.a1, i, 1);
            this.genStoreVarWord(this.a1, i, 0);
            if (i % 2) this.op_empty();
        }
        this.op("ret");
    }
}

class Gen_rawCopyS2L extends GenBase {
    constructor(width, space) {
        super(width, space, "rawCopyS2L", 4, 0);

        this.q1 = cyclicCopy(this.assignRegs(6), this.width);
        this.r1 = cyclicCopy(this.assignRegs(6), this.width);

        this.generate();
    }

    generate() {
        let label = this.makeLabel("adjust_neg");

        this.op("cmp", "x1", "xzr");
        this.op("b.lt", label);
        this.op_empty();

        let a1 = expand(new RegVar(1), 31, this.width);
        this.genStoreVar(a1, 0);
        this.op("ret");
        this.op_empty();

        this.op_label(label);
        this.genPositive();
        this.op("ret");
    }

    genPositive() {
        if (this.width > 1) {
            this.op("mov", "x2", "-1");
        }

        this.op("adr", "x3", this.getVarName("rawq"));
        this.op_empty();

        let a1 = expand(new RegVar(1), 2, this.width);

        let j = 0;
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.q1, i, 3);
            this.genAddWord(this.r1, a1, this.q1, i);
            if (i > 0)
                this.genStoreVarWord(this.r1, j++, 0);
            if (i % 2) this.op_empty();
        }
        this.genStoreVarWord(this.r1, j, 0);
        if (this.width % 2) this.op_empty();
    }
}

class Gen_rawCmp extends GenBase {
    constructor(width, space) {
        super(width, space, "rawCmp", 3, 0);

        this.a1 = cyclicCopy(this.assignRegs(4), this.width);
        this.b1 = cyclicCopy(this.assignRegs(4), this.width);

        this.generate();
    }

    generate() {
        this.genCmp();
        this.op("cneg", "x0", "x2", "lo");
        this.op("ret");
    }


    genCmp() {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.a1, i, 0);
            this.genLoadVarWord(this.b1, i, 1);
            this.genSubWord(this.a1, this.a1, this.b1, i);
            this.genAccum(i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    genAccum(i) {
        if (i === 0) {
            this.op("cset", "x2", "ne");
        } else {
            this.op("cinc", "x2", "x2", "ne");
        }
    }
}

class Gen_rawBinOp extends GenBase {
    constructor(width, space, name, binOp) {
        super(width, space, name, 4, 4);

        assert(width <= 12);

        this.binOp = binOp;
        this.unary = this. isUnary(binOp);

        this.r1 = this.assignRegs(this.width);
        this.b1 = cyclicCopy(this.workRegs, this.width);
        this.releaseRegs(this.workRegs);
        this.r2 = this.assignRegs(this.width);
        this.generate();
    }

    generate() {
        this.pushRegs();

        this.genBinaryOp();

        this.op("adr", "x3", this.getVarName("rawq"));
        this.genSubLoad(this.r2, this.r1, this.r2, 3);
        this.genCselStore(this.r1, this.r2, this.r1, 0);

        this.popRegs();
        this.op("ret");
    }


    genBinaryOp() {
        if (this.unary)
            this.genBinaryOp1();
        else
            this.genBinaryOp2();

        this.genLboMask(this.r1[this.width-1], new Reg(2));
    }

    genBinaryOp1() {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.r1, i, 1);
            this.op(this.binOp, this.r1[i], this.r1[i]);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    genBinaryOp2() {

        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.r1, i, 1);
            this.genLoadVarWord(this.b1, i, 2);
            this.op(this.binOp, this.r1[i], this.r1[i], this.b1[i]);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    isUnary(binOp) {
        return (binOp === "mvn");
    }
}

class Gen_rawAdd extends GenBase {
    constructor(width, space, name, shortOper2) {
        super(width, space, name, 4, 4);

        assert(width <= 12);

        this.shortOper2 = shortOper2;

        this.r1 = this.assignRegs(this.width);
        this.b1 = cyclicCopy(this.workRegs, this.width);
        this.releaseRegs(this.workRegs);
        this.r2 = this.assignRegs(this.width);
        this.generate();
    }

    generate() {
        this.pushRegs();

        if (this.shortOper2)
            this.genAdditionOpLS();
        else
            this.genAdditionOp();

        this.op("cset", "x2", "cs");
        this.op_empty();

        this.genSubtraction();
        this.genResultStore();

        this.op_empty();
        this.op_label(this.makeLabel("out"));
        this.popRegs();
        this.op("ret");
        this.op_empty();
    }


    genAdditionOp() {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.r1, i, 1);
            this.genLoadVarWord(this.b1, i, 2);
            this.genAddWord(this.r1, this.r1, this.b1, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    genAdditionOpLS() {
        let b = expand(new RegVar(2), 31, this.width);

        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.r1, i, 1);
            this.genAddWord(this.r1, this.r1, b, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    genSubtraction() {
        this.op("adr", "x3", this.getVarName("rawq"));
        this.genSubLoad(this.r2, this.r1, this.r2, 3);
    }

    genResultStore() {
        const label = this.makeLabel("done_s");

        this.op("cbnz", "x2", label);
        this.op("b.hs",       label);
        this.op_empty();

        this.genStoreVar(this.r1, 0);
        this.op_empty();
        this.op("b", this.makeLabel("out"));
        this.op_empty();
        this.op_label(label);
        this.genStoreVar(this.r2, 0);
    }
}

class Gen_rawSub extends GenBase {
    constructor(width, space, name, paramsType) {
        super(width, space, name, 4, 4);

        assert(width <= 20);

        this.paramsType = paramsType;

        this.r1 = this.assignRegs(this.width);
        this.b1 = cyclicCopy(this.workRegs, this.width);
        this.releaseRegs(this.workRegs);
        this.r2 = this.assignRegs(this.width);
        this.generate();
    }

    generate() {
        this.pushRegs();

        if (this.paramsType === "LS")
            this.genSubtractionOpLS();
        else if (this.paramsType === "SL")
            this.genSubtractionOpSL();
        else
            this.genSubtractionOp();

        const doneLabel = this.makeLabel("done");
        this.op("b.cs", doneLabel);
        this.op_empty();

        this.genAddition();

        this.op_label(doneLabel);
        this.genStoreVar(this.r1, 0);

        this.popRegs();
        this.op("ret");
        this.op_empty();
    }


    genSubtractionOp() {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.r1, i, 1);
            this.genLoadVarWord(this.b1, i, 2);
            this.genSubWord(this.r1, this.r1, this.b1, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    genSubtractionOpLS() {
        let b = expand(new RegVar(2), 31, this.width);

        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.r1, i, 1);
            this.genSubWord(this.r1, this.r1, b, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }

    genSubtractionOpSL() {
        let a = expand(new RegVar(1), 31, this.width);
        this.genSubLoad(this.r1, a, this.r1, 2);
    }

    genAddition() {
        this.op("adr", "x3", this.getVarName("rawq"));

        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.b1, i, 3);
            this.genAddWord(this.r1, this.r1, this.b1, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();
    }
}

class Gen_rawSubRegular extends GenBase {
    constructor(width, space, name) {
        super(width, space, name, 4, 0);

        this.a1 = cyclicCopy(this.assignRegs(4), this.width);
        this.b1 = cyclicCopy(this.assignRegs(4), this.width);

        this.generate();
    }

    generate() {
        let j = 0;
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.a1, i, 1);
            this.genLoadVarWord(this.b1, i, 2);
            this.genSubWord(this.a1, this.a1, this.b1, i);
            if (i > 0)
                this.genStoreVarWord(this.a1, j++, 0);
            if (i % 2) this.op_empty();
        }

        this.genStoreVarWord(this.a1, j, 0);
        if (this.width % 2) this.op_empty();

        this.op("ret");
        this.op_empty();
    }
}

class Gen_rawNeg extends GenBase {
    constructor(width, space, name) {
        super(width, space, name, 4, 4);

        assert(width <= 20);

        this.r1 = this.assignRegs(this.width);
        this.b1 = cyclicCopy(this.workRegs, this.width);

        this.generate();
    }

    generate() {
        this.pushRegs();

        this.genIsZero();

        const doneLabel = this.makeLabel("done_zero");
        this.op("cbz", "x2", doneLabel);
        this.op_empty();

        this.genSubtraction();

        if (this.hasSavedRegs())
           this.op("b", this.makeLabel("out"));
        else
           this.op("ret");
        this.op_empty();

        this.op_label(doneLabel);
        this.genZeroStore();
        this.op_empty();

        if (this.hasSavedRegs())
            this.op_label(this.makeLabel("out"));

        this.popRegs();
        this.op("ret");
        this.op_empty();
    }


    genIsZero() {
        this.op("mov", "x2", "xzr");

        let i = 0;
        for (; i < this.width - 1; i += 2) {
            let r = this.getNextWorkReg();

            this.op("ldp", this.r1[i], this.r1[i+1], this.getMemWord(1, i));
            this.op("orr", r, this.r1[i], this.r1[i+1]);
            this.op("orr", "x2", "x2", r);
            this.op_empty();
        }

        if (this.width % 2) {
            this.op("ldr", this.r1[i], this.getMemWord(1, i));
            this.op("orr", "x2", "x2", this.r1[i]);
            this.op_empty();
        }
    }

    genSubtraction() {
        this.op("adr", "x3", this.getVarName("rawq"));

        let j = 0;
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.b1, i, 3);
            this.genSubWord(this.r1, this.b1, this.r1, i);
            if (i > 0)
                this.genStoreVarWord(this.r1, j++, 0);
            if (i % 2) this.op_empty();
        }
        this.genStoreVarWord(this.r1, j, 0);
        if (this.width % 2) this.op_empty();
    }

    genZeroStore() {
        let zero = cyclicCopy(new RegVar(31), this.width);
        this.genStoreVar(zero, 0);
    }
}

class Gen_rawNegLS extends GenBase {
    constructor(width, space, name) {
        super(width, space, name, 4, 4);

        assert(width <= 12);

        this.r1 = this.assignRegs(this.width);
        this.a2 = cyclicCopy(this.workRegs, this.width);
        this.releaseRegs(this.workRegs);
        this.r2 = this.assignRegs(this.width);
        this.generate();
    }

    generate() {
        this.pushRegs();

        this.genSubtractionOpQ();
        this.op_empty();
        this.genSubtractionOpA();
        this.op_empty();

        this.genAdd(this.r2, this.r2, this.r1);
        this.op_empty();

        this.op_label(this.makeLabel("done"));
        this.genStoreVar(this.r2, 0);

        this.popRegs();
        this.op("ret");
        this.op_empty();
    }


    genSubtractionOpQ() {
        this.op("adr", "x3", this.getVarName("rawq"));

        let c = expand(new RegVar(2), 31, this.width);

        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.r1, i, 3);
            this.genSubWord(this.r2, this.r1, c, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();

        this.op("cset", "x2", "cs");
    }

    genSubtractionOpA() {
        for (let i = 0; i < this.width; i++) {
            this.genLoadVarWord(this.a2, i, 1);
            this.genSubWord(this.r2, this.r2, this.a2, i);
            if (i % 2) this.op_empty();
        }
        if (this.width % 2) this.op_empty();

        this.op("cset", "x3", "cs");
        this.op("orr",  "x3", "x3", "x2");
        this.op_empty();
        this.op("cbz", "x3", this.makeLabel("done"));
    }
}

class Gen_rawShr extends GenBase {
    constructor(width, space, name) {
        super(width, space, name, 6, 2);

        assert(width <= 12);

        this.r1 = this.assignRegs(this.width);
        this.releaseRegs(this.workRegs);
        this.c1 = cyclicCopy(this.workRegs, this.width);

        this.generate();
    }

    generate() {
        if (this.width === 1) {
            this.op("ldr", "x3", "[x1]");
            this.op("lsr", "x3", "x3", "x2");
            this.op("str", "x3", "[x0]");
            this.op("ret");
            return;
        }

        this.pushRegs();
        this.genLoadVar(this.r1, 1);
        this.op_empty();

        this.genCalcShift();
        this.genCalcJump();

        this.genShift();

        if (this.hasSavedRegs()) {
            this.op_label(this.makeLabel("done"));
            this.popRegs();
            this.op("ret");
            this.op_empty();
        }

        if (this.width > 2) this.genJumpTable();
        this.op_empty();
    }


    genCalcShift() {
        this.op("and", "x3", "x2", "0x3f");
        this.op("mov", "x4", "0x3f");
        this.op("sub", "x4", "x4", "x3");
        this.op_empty();
    }

    genCalcJump() {
        if (this.width > 2) {
            this.op("lsr", "x2", "x2", "#6");
            this.op("adr", "x5", this.makeLabel("word_shift"));
            this.op("ldr", "x5", "[x5, x2, lsl 3]");
            this.op("br",  "x5");

        } else {
            this.op("tbnz", "x2", "6", this.makeLabel("word_shift_1"));
        }
        this.op_empty();
    }

    genJumpTable() {
        this.op_label(this.makeLabel("word_shift"));

        for (let i = 0; i < this.width; i++) {
            this.op(".quad", this.makeWordLabel(i));
        }
    }

    genShift() {
        let i = 0;
        for (; i < this.width-1; i++) {
            this.op_label(this.makeWordLabel(i));

            this.genBitShift(i);
            this.genStore(i);

            if (this.hasSavedRegs())
                this.op("b", this.makeLabel("done"));
            else
                this.op("ret");
            this.op_empty();
        }

        this.op_label(this.makeWordLabel(i));

        this.genBitShift(i);
        this.genStore(i);

        if (!this.hasSavedRegs())
            this.op("ret");

         this.op_empty();
    }

    genBitShift(wordShift) {
        assert(wordShift < this.width);

        let i = wordShift;
        for (; i < this.width - 1; i++) {
            let c0 = this.getNextWorkReg();

            this.op("lsr",  this.r1[i], this.r1[i],   "x3");
            this.op("lsl",  c0,      this.r1[i+1], "x4");
            this.op("orr",  this.r1[i], this.r1[i],    c0, "lsl #1");
            this.op_empty();
        }

        this.op("lsr", this.r1[i], this.r1[i], "x3");
        this.op_empty();
    }

    genStore(wordShift) {
        assert(wordShift < this.width);

        let a = new RegVar();
        for (let i = 0; i < this.width; i++) {
            const j = wordShift + i;
            a.push(j < this.width ? this.r1[j] : new Reg(31));
        }

        this.genStoreVar(a, 0);
    }

    makeWordLabel(i) {
        return this.makeLabel("word_shift") + "_" + i;
    }
}

class Gen_rawShl extends GenBase {
    constructor(width, space, name) {
        super(width, space, name, 7, 2);

        assert(width <= 12);

        this.r1 = this.assignRegs(this.width);
        this.releaseRegs(this.workRegs);
        this.releaseRegs(new RegVar(2, 3, 4, 5));
        this.r2 = this.assignRegs(this.width);
        this.generate();
    }

    generate() {
        if (this.width === 1) {
            this.op("ldr", "x3", "[x1]");
            this.op("lsl", "x3", "x3", "x2");
            this.op("str", "x3", "[x0]");
            this.op("ret");
            return;
        }

        this.pushRegs();
        this.genLoadVar(this.r1, 1);
        this.op_empty();

        this.genCalcShift();
        this.genCalcJump();

        this.genShift();

        this.op_label(this.makeLabel("sub"));
        this.genLboMask(this.r1[this.width-1], new Reg(6));

        this.op("adr", "x1", this.getVarName("rawq"));
        this.genSubLoad(this.r2, this.r1, this.r2, 1);
        this.genCselStore(this.r1, this.r2, this.r1, 0);

        this.popRegs();
        this.op("ret");

        if (this.width > 2) this.genJumpTable();
        this.op_empty();
    }


    genCalcShift() {
        this.op("and", "x3", "x2", "0x3f");
        this.op("mov", "x4", "0x3f");
        this.op("sub", "x4", "x4", "x3");
        this.op_empty();
    }

    genCalcJump() {
        if (this.width > 2) {
            this.op("lsr", "x2", "x2", "#6");
            this.op("adr", "x5", this.makeLabel("word_shift"));
            this.op("ldr", "x5", "[x5, x2, lsl 3]");
            this.op("br",  "x5");

        } else {
            this.op("tbnz", "x2", "6", this.makeLabel("word_shift_1"));
        }
        this.op_empty();
    }

    genJumpTable() {
        this.op_label(this.makeLabel("word_shift"));

        for (let i = 0; i < this.width; i++) {
            this.op(".quad", this.makeWordLabel(i));
        }
    }

    genShift() {
        let i = 0;
        for (; i < this.width; i++) {
            this.op_label(this.makeWordLabel(i));
            this.genBitShift(i);

            if (i !== this.width -1)
            {
                this.op("b", this.makeLabel("sub"));
                this.op_empty();
            }
        }
    }

    genBitShift(wordShift) {
        assert(wordShift < this.width);

        let i = this.width - 1 - wordShift;
        let j = this.width - 1;
        for (; i > 0; i--, j--) {
            let c0 = this.getNextWorkReg();

            this.op("lsl",  this.r1[j], this.r1[i],   "x3");
            this.op("lsr",  c0,      this.r1[i-1], "x4");
            this.op("orr",  this.r1[j], this.r1[j],   c0, "lsr #1");
            this.op_empty();
        }

        this.op("lsl", this.r1[j--], this.r1[0], "x3");

        for (; j >= 0; j--) {
            this.op("mov", this.r1[j], "xzr");
        }
        this.op_empty();
    }

    makeWordLabel(i) {
        return this.makeLabel("word_shift") + "_" + i;
    }
}

class Gen_rawMul extends GenBase {
    constructor(width, space, name, canOptimizeConsensys) {
        super(width, space, name, 9, 0);

        assert(width <= 12);

        this.isShort = (width <= 6);
        this.canOptimizeConsensys = canOptimizeConsensys;

        if (this.isShort) {
            this.r1 = this.assignRegs(this.width+1);
            this.b1 = this.assignRegs(this.width);
            this.q1 = this.assignRegs(this.width);
            this.r2 = this.r1.slice(1, this.width+1);
            this.q2 = this.q1;
            this.b2 = this.b1;
        } else {
            this.r1 = this.assignRegs(this.width+1);
            this.workRegs = this.assignRegs(4);
            this.b1 = cyclicCopy(this.workRegs, this.width);
            this.q1 = cyclicCopy(this.workRegs, this.width);

            this.releaseRegs(this.workRegs);
            this.releaseRegs(new RegVar(3,4,5,7));

            this.r2 = this.r1.slice(1, this.width+1);
            this.releaseRegs(removeSlice(this.r1, 1, this.width));
            this.q2 = cyclicCopy(new RegVar(1,2), this.width);
            this.b2 = this.assignRegs(this.width);
        }
        this.generate();
    }

    generate() {
        this.pushRegs();

        if (this.isShort) {
            this.genLoadVar(this.b1, 2);
            this.op_empty();
        }

        this.op("adr", "x4", this.getVarName("np"));
        this.op("ldr", "x4", this.getMem(4, 0));
        this.op_empty();

        this.op("adr", "x6", this.getVarName("rawq"));
        if (this.isShort) {
            this.genLoadVar(this.q1, 6);
        }
        this.op_empty();

        this.genMul();

        this.op_comment("result ge ", this.getVarName("rawq"));
        if (this.isShort)
            this.genSub(this.b2, this.r2, this.q2);
        else
            this.genSubLoad(this.b2, this.r2, this.q2, 6);
        this.op_empty();

        if (!this.canOptimizeConsensys) {
            this.op("cinc", "x8", "x8", "hs");
            this.op("cmp", "x8", "1");
            this.op_empty();
        }

        this.genCselStore(this.r2, this.b2, this.r2, 0);

        this.popRegs();
        this.op("ret");
        this.op_empty();
    }

    genMul() {
        for (let i = 0; i < this.width; i++) {
            if (i === 0) {
                this.op_comment("product", i, " = pRawB * pRawA[", i, "]");
            } else {
                this.op_comment("product", i, " = product", i-1, " + pRawB * pRawA[", i, "]");
            }

            this.op("ldr", "x3", this.getMemWord(1, i));

            if (i === 0) {
                this.genMulAB(i);
            } else {
                this.genAddMulAB(i);
            }

            this.op_comment("np0 = Fq_np * product", i, "[0]");
            this.op("mul", "x5", "x4", this.r1[0]);
            this.op_empty();

            this.op_comment("product", i, " = product", i, " + Fq_rawq * np0");
            this.genMulRawq(i);
        }
    }

    genMulAB(w) {
        let i = 0;
        for(; i < this.width; i++) {
            let ra = new Reg(3);
            let rs = new Reg(7);

            if (!this.isShort) this.genLoadVarWord(this.b1, i, 2);

            this.op("mul", (i ? rs : this.r1[i]), this.b1[i], ra);
            if (i)
                this.genAddReg(this.r1[i], this.r1[i], rs, i-1);

            this.op("umulh", this.r1[i+1], this.b1[i], ra);

        }
        if (i > 1)
            this.op("adc", this.r1[i], this.r1[i], "xzr");
        this.op_empty();
    }

    genAddMulAB(w) {
        let ra = new Reg(3);

        let i = 0;
        for(; i < this.width; i++) {
            if (!this.isShort) this.genLoadVarWord(this.b1, i, 2);
            this.op("mul",  this.r1[i], this.b1[i], ra);
            this.genAddReg(this.r1[i], this.r1[i], this.r1[i+1], i);
        }

        if (!this.canOptimizeConsensys) {
            this.op("adcs", this.r1[i], "xzr", "x8");
            this.op("adc",  "x8", "xzr", "xzr");
            this.op_empty();
        } else {
            this.op("adc", this.r1[i], "xzr", "xzr");
            this.op_empty();
            this.op("adds", this.r1[1], this.r1[1], "x5");
        }

        let t1 = cyclicCopy(new RegVar(7, 5), this.width);

        for(i = 0; i < this.width; i++) {
             let iAdd = (i === this.width - 1) ? "adc" : "adcs";

             if (!this.canOptimizeConsensys) {
                iAdd = (i) ? "adcs" : "adds";
             }

            if (!this.isShort) this.genLoadVarWord(this.b1, i, 2);
            this.op("umulh",  t1[i],   this.b1[i],   ra);
            this.op(iAdd,   this.r1[i+1], this.r1[i+1], t1[i]);
        }

        if (!this.canOptimizeConsensys) {
            this.op("adc",  "x8", "x8", "xzr");
        }
        this.op_empty();
    }

    genMulRawq(w) {
        let t1 = cyclicCopy(new RegVar(7, 3), this.width);
        let np0 = new Reg(5);

        let i = 0;
        for(; i < this.width; i++) {
            if (!this.isShort) this.genLoadVarWord(this.q1, i, 6);
            this.op("mul",    t1[i], this.q1[i], np0);
            this.genAddReg(this.r1[i], this.r1[i], t1[i], i);
        }
        if (!this.canOptimizeConsensys) {
            this.op("adcs", this.r1[i], this.r1[i], "xzr");
            if (w) {
                this.op("adc",  "x8", "x8", "xzr");
            } else {
                this.op("adc",  "x8", "xzr", "xzr");
            }

        } else {
            this.op("adc", this.r1[i], this.r1[i], "xzr");
        }
        this.op_empty();

        for(i = 0; i < this.width; i++) {
            if (!this.isShort) this.genLoadVarWord(this.q1, i, 6);
            this.op("umulh",  t1[i],   this.q1[i],   np0);
            this.genAddReg(this.r1[i+1], this.r1[i+1], t1[i], i);
        }

        if (!this.canOptimizeConsensys) {
            this.op("adc",  "x8", "x8", "xzr");
        } else
        {
            if (w < this.width - 1) {
                this.op("adc", np0, "xzr", "xzr");
            }
        }
        this.op_empty();
    }
}

class Gen_rawMul1 extends GenBase {
    constructor(width, space, name, mulAB, canOptimizeConsensys) {
        super(width, space, name, 9, 0);

        assert(width <= 12);

        this.isShort = (width <= 6);
        this.mulAB = mulAB;
        this.canOptimizeConsensys = canOptimizeConsensys;

        if (this.isShort) {
            this.r1 = this.assignRegs(this.width+1);
            this.b1 = this.assignRegs(this.width);
            this.q1 = this.assignRegs(this.width);
            this.r2 = this.r1.slice(1, this.width+1);
            this.q2 = this.q1;
            this.b2 = this.b1;
        } else {
            this.r1 = this.assignRegs(this.width+1);
            this.workRegs = this.assignRegs(4);
            this.b1 = cyclicCopy(this.workRegs, this.width);
            this.q1 = cyclicCopy(this.workRegs, this.width);

            this.releaseRegs(this.workRegs);
            this.releaseRegs(new RegVar(3,4,5,7));

            this.r2 = this.r1.slice(1, this.width+1);
            this.releaseRegs(removeSlice(this.r1, 1, this.width));

            this.q2 = cyclicCopy(new RegVar(1,2), this.width);
            this.b2 = this.assignRegs(this.width);
        }
        this.generate();
    }

    generate() {
        this.pushRegs();

        if (this.mulAB) {
            if (this.isShort) {
                this.genLoadVar(this.b1, 1);
            }
        } else {
            this.genLoadVar(this.r1, 1);

            let lastR = this.r1[this.width];
            this.op("mov", lastR, "xzr");
        }
        this.op_empty();


        this.op("adr", "x4", this.getVarName("np"));
        this.op("ldr", "x4", this.getMem(4, 0));
        this.op_empty();

        this.op("adr", "x6", this.getVarName("rawq"));
        if (this.isShort) {
            this.genLoadVar(this.q1, 6);
        }
        this.op_empty();

        if (this.mulAB) {
            this.op_comment("product0 = pRawB * pRawA");
            this.genMulAB(0);
        }

        for (let i = 0; i < this.width; i++) {
            this.op_comment("np0 = Fq_np * product", i, "[0]");
            this.op("mul", "x5", "x4", this.r1[i ? 1 : 0]);

            this.op_comment("product", i, " = product", i, " + Fq_rawq * np0");
            this.genMulRawq(i);
        }

        this.op_comment("result ge ", this.getVarName("rawq"));
        if (this.isShort)
            this.genSub(this.b2, this.r2, this.q2);
        else
            this.genSubLoad(this.b2, this.r2, this.q2, 6);
        this.op_empty();

        if (!this.canOptimizeConsensys) {
            this.op("cinc", "x8", "x8", "hs");
            this.op("cmp", "x8", "1");
            this.op_empty();
        }

        this.genCselStore(this.r2, this.b2, this.r2, 0);

        this.popRegs();
        this.op("ret");
        this.op_empty();
    }

    genMulAB(w) {
        let i = 0;
        for(; i < this.width; i++) {
            let ra = new Reg(2);
            let rs = new Reg(7);

            if (!this.isShort) this.genLoadVarWord(this.b1, i, 1);

            this.op("mul", (i ? rs : this.r1[i]), this.b1[i], ra);
            if (i)
                this.genAddReg(this.r1[i], this.r1[i], rs, i-1);

            this.op("umulh", this.r1[i+1], this.b1[i], ra);

        }
        if (i > 1)
            this.op("adc", this.r1[i], this.r1[i], "xzr");
        this.op_empty();
    }

    genMulRawq(w) {
        let t1 = cyclicCopy(new RegVar(7, 3), this.width);
        let np0 = new Reg(5);
        let carry = new Reg(8);

        let i = 0;
        for(; i < this.width; i++) {
            if (!this.isShort) this.genLoadVarWord(this.q1, i, 6);
            this.op("mul",    t1[i], this.q1[i], np0);
            if (w > 0)
                this.genAddReg(this.r1[i], this.r1[i+1], t1[i], i);
            else
                this.genAddReg(this.r1[i], this.r1[i], t1[i], i);
        }
        if (w > 0) {
            if (!this.canOptimizeConsensys) {
                this.op("adcs", this.r1[i], "xzr", carry);
                this.op("adc", carry, "xzr", "xzr");
                this.op_empty();
            } else {
                this.op("adc", this.r1[i], "xzr", "xzr");
                this.op_empty();
                this.op("adds", this.r1[1], this.r1[1], carry);
            }
        } else {
            this.op("adc", this.r1[i], this.r1[i], "xzr");
            this.op_empty();
        }

        for(i = 0; i < this.width; i++) {
            if (!this.isShort) this.genLoadVarWord(this.q1, i, 6);
            this.op("umulh",  t1[i],   this.q1[i],   np0);
            this.genAddReg(this.r1[i+1], this.r1[i+1], t1[i], i);
        }

        if (!this.canOptimizeConsensys) {
            if (w) {
                this.op("adc", carry, carry, "xzr");
            } else {
                this.op("adc", carry, "xzr", "xzr");
            }
        } else {
            if (w < this.width - 1) {
                this.op("adc", carry, "xzr", "xzr");
            }
        }
        this.op_empty();
    }
}

function generate(width, space, canOptimizeConsensys) {
    let generators = [
        new Gen_rawAdd(width, space, "rawAdd", false),
        new Gen_rawAdd(width, space, "rawAddLS", true),
        new Gen_rawSub(width, space, "rawSub", ""),
        new Gen_rawSub(width, space, "rawSubSL", "SL"),
        new Gen_rawSub(width, space, "rawSubLS", "LS"),
        new Gen_rawSubRegular(width, space, "rawSubRegular"),
        new Gen_rawNeg(width, space, "rawNeg"),
        new Gen_rawNegLS(width, space, "rawNegLS"),
        new Gen_rawMul(width, space, "rawMMul", canOptimizeConsensys),
        new Gen_rawMul1(width, space, "rawMMul1", true, canOptimizeConsensys),
        new Gen_rawMul1(width, space, "rawFromMontgomery", false, canOptimizeConsensys),
        new Gen_rawIsZero(width, space),
        new Gen_rawIsEq(width, space),
        new Gen_rawCmp(width, space),
        new Gen_rawCopy(width, space),
        new Gen_rawCopyS2L(width, space),
        new Gen_rawSwap(width, space),
        new Gen_rawBinOp(width, space, "rawAnd", "and"),
        new Gen_rawBinOp(width, space, "rawOr",  "orr"),
        new Gen_rawBinOp(width, space, "rawXor", "eor"),
        new Gen_rawBinOp(width, space, "rawNot", "mvn"),
        new Gen_rawShr(width, space, "rawShr"),
        new Gen_rawShl(width, space, "rawShl")
    ];
    return generators.join("\n") + "\n";
}

function genFuncs(space, q) {
    const n64 = Math.floor((q.bitLength() - 1) / 64)+1;
    const canOptimizeConsensys = q.shiftRight((n64-1)*64).leq( bigInt.one.shiftLeft(64).minus(1).shiftRight(1).minus(1) );

    return generate(n64, space, canOptimizeConsensys);
}

