
const assert = require("assert");

module.exports = class RegManager {
    constructor(name, neededReges) {

        this.availableRegs = ["rax", "r8", "r9", "r10", "r11"];
        this.pushableRegs = ["r12", "r13", "r14", "r15", "rbp", "rbx"];
        this.allRegs = ["rax", "rbx", "rcx", "rdx", "rdi", "rsi", "rsp", "rbp", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"];

        this.instructions= {
            "adcx": ["IOR", "I"],
            "adox": ["IOR", "I"],
            "shl": ["IOR", "I"],
            "shr": ["IOR", "I"],
            "sub": ["IO", "I"],
            "sbb": ["IO", "I"],
            "cmp": ["I", "I"],
            "test": ["I", "I"],
            "mov": ["O", "I"],
            "mulx": ["OR","OR", "I"],
            "xor": ["IO", "O"],
        };

        this.code = [];

        this.name = name;
        this.neededRegs = neededReges;
        this.regRefs = [];

        this.wrAvailable;
        this.wrAssignments = [];
        this.pushedRegs = [];
    
        if (this.neededRegs > this.availableRegs.length + this.pushableRegs.length) {
            this.nUsedRegs = this.availableRegs.length + this.pushableRegs.length -1;
            this.wrAvailable = [this.pushableRegs[this.pushableRegs.length-1]];
            this.pushedRegs = this.pushableRegs;
        } else {
            this.nUsedRegs = this.neededRegs;
            this.wrAvailable = null;
            this.pushedRegs = this.pushableRegs.slice(0, this.nUsedRegs - this.availableRegs.length);
        }
        for (let i=0; i<this.neededRegs; i++) {
            if (i < this.availableRegs.length) {
                this.regRefs[i] = this.availableRegs[i];
            } else if (i<this.nUsedRegs) {
                this.regRefs[i] = this.pushableRegs[i-this.availableRegs.length];
            } else {
                this.regRefs[i] = `[rsp + ${(i-this.nUsedRegs)*8  }]`;
            }
        }
    }

    _addHeader() {
        if (this.neededRegs>this.nUsedRegs) {
            this.code.unshift(`    sub rsp, ${(this.neededRegs-this.nUsedRegs)*8}`);
        }
        for (let i=0; i<this.pushedRegs.length; i++) {
            this.code.unshift(`    push ${this.pushedRegs[i]}`);
        }
    }

    _addgetFooter() {
        if (this.neededRegs>this.nUsedRegs) {
            this.code.push(`    add rsp, ${(this.neededRegs-this.nUsedRegs)*8}`);
        }
        for (let i=0; i<this.pushedRegs.length; i++) {
            this.code.push(`    pop ${this.pushedRegs[i]}`);
        }
    }

    _indexOfWrAssignment(ref) {
        for (let i=0; i<this.wrAssignments.length; i++) {
            if (this.wrAssignments[i].ref == ref) return i;
        }
        return -1;
    }

    flushWr(removeAssignments) {
        for (let i=this.wrAssignments.length-1; i>=0; i--) {
            if (this.wrAssignments[i].modified) {
                this.code.push(`    mov ${this.regRefs[this.wrAssignments[i].ref]}, ${this.wrAssignments[i].reg}`);
                this.wrAssignments[i].modified = false;
            }
            if (removeAssignments) {
                this.wrAvailable.push(this.wrAssignments[i].reg);
                this.wrAssignments.pop();
            }
        }
    }


    _loadWr(ref, loadValue) {
        const idx = this._indexOfWrAssignment(ref);
        if (idx>=0) {
            this.wrAssignments.push(this.wrAssignments.splice(idx, 1)[0]); // Move it to the end so it will be removed last
        } else {
            // If not available registers flush and remove the first assignemnt
            if (this.wrAvailable.length == 0) {
                if (this.wrAssignments[0].modified) {
                    this.code.push(`    mov ${this.regRefs[this.wrAssignments[0].ref]}, ${this.wrAssignments[0].reg}`);
                }
                this.wrAvailable.push(this.wrAssignments[0].reg);
                this.wrAssignments.shift();
            }
            const a = {
                reg: this.wrAvailable.shift(),
                ref: ref,
            };
            this.wrAssignments.push(a);
            if (loadValue) {
                this.code.push(`    mov ${a.reg}, ${this.regRefs[a.ref]}`);
            }
        }

        return this.wrAssignments[this.wrAssignments.length-1];
    }


    op(instructionName, ...args) {


        if (! this.instructions[instructionName] ) {
            console.log("Instruction not defined: ", instructionName);
            assert(false );
        }

        const inst = this.instructions[instructionName];

        if (inst.length != args.length) {
            console.log("Invalid number of params", instructionName);
            assert(false );
        }

        let dUsed = -1;
        let dLoad = -1;
        for (let i=0; i < inst.length; i++) {
            if ((typeof args[i] === "string")&&(this.allRegs.indexOf(args[i]) < 0)) {
                if (dUsed>=0) {
                    if (dLoad>=0) {
                        console.log("tryning to load two args");
                        assert(false);
                    } else {
                        dLoad = dUsed;
                        dUsed = i;
                    }
                }
                dUsed = i;
            } else {
                if (args[i] >= this.nUsedRegs) {
                    if ((dUsed>=0 || inst[i].indexOf("R") >= 0)) {
                        if (dLoad>=0) {
                            console.log("tryning to load two args");
                            assert(false);
                        }
                        dLoad = i;
                    } else {
                        dUsed = i;
                    }
                }
            }
        }

        const params = [];
        for (let i=0; i < inst.length; i++) {
            if (typeof args[i] === "string") {
                params.push(args[i]);
            } else if (args[i]<this.nUsedRegs) {
                params.push(this.regRefs[args[i]]);
            } else if ((dLoad>=0)&&(args[i] == args[dLoad])) {
                params.push(this._loadWr(args[i], inst[i].indexOf("I")>=0).reg);
                if (inst[i].indexOf("O")>=0) {
                    this.wrAssignments[this._indexOfWrAssignment(args[i])].modified = true;
                }
            } else {
                params.push(this.regRefs[args[i]]);
            }
        }
        this.code.push(`    ${instructionName} ${params.join(",")}`);
    }

    getCode() {

        this._addHeader();
        this._addgetFooter();
        this.code.unshift(`${this.name}:`);
        this.code.push("    ret");

        return this.code.join("\n");
    }

};