const tester = require("./tester/buildzqfieldtester.js");

const ZqField = require("ffjavascript").ZqField;

const bigInt = require("big-integer");

const bn128q = new bigInt("21888242871839275222246405745257275088696311157297823662689037894645226208583");
const bn128r = new bigInt("21888242871839275222246405745257275088548364400416034343698204186575808495617");
const bls12_381q = new bigInt("4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787");
const secp256k1q = new bigInt("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F", 16);
const secp256k1r = new bigInt("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141", 16);
const mnt6753q = new bigInt("41898490967918953402344214791240637128170709919953949071783502921025352812571106773058893763790338921418070971888458477323173057491593855069696241854796396165721416325350064441470418137846398469611935719059908164220784476160001");
const mnt6753r = new bigInt("41898490967918953402344214791240637128170709919953949071783502921025352812571106773058893763790338921418070971888253786114353726529584385201591605722013126468931404347949840543007986327743462853720628051692141265303114721689601");
const gl = new bigInt("FFFFFFFF00000001", 16);

describe("field asm test", function () {
    this.timeout(1000000000);
    function generateTest(curve, name) {
        it(name + " add", async () => {
            const tv = buildTestVector2(curve, "add");
            await tester(curve, tv);
        });
        it(name + " sub", async () => {
            const tv = buildTestVector2(curve, "sub");
            await tester(curve, tv);
        });
        it(name + " neg", async () => {
            const tv = buildTestVector1(curve, "neg");
            await tester(curve, tv);
        });
        it(name + " square", async () => {
            const tv = buildTestVector1(curve,"square");
            await tester(curve, tv);
        });
        it(name + " mul", async () => {
            const tv = buildTestVector2(curve, "mul");
            await tester(curve, tv);
        });
        it(name + " eq", async () => {
            const tv = buildTestVector2(curve, "eq");
            await tester(curve, tv);
        });
        it(name + " neq", async () => {
            const tv = buildTestVector2(curve, "neq");
            await tester(curve, tv);
        });
        it(name + " lt", async () => {
            const tv = buildTestVector2(curve, "lt");
            await tester(curve, tv);
        });
        it(name + " gt", async () => {
            const tv = buildTestVector2(curve, "gt");
            await tester(curve, tv);
        });
        it(name + " leq", async () => {
            const tv = buildTestVector2(curve, "leq");
            await tester(curve, tv);
        });
        it(name + " geq", async () => {
            const tv = buildTestVector2(curve, "geq");
            await tester(curve, tv);
        });
        it(name + " logical and", async () => {
            const tv = buildTestVector2(curve,"land");
            await tester(curve, tv);
        });
        it(name + " logical or", async () => {
            const tv = buildTestVector2(curve, "lor");
            await tester(curve, tv);
        });
        it(name + " logical not", async () => {
            const tv = buildTestVector1(curve,"lnot");
            await tester(curve, tv);
        });
        it(name + " idiv", async () => {
            const tv = buildTestVector2(curve,"idiv");
            await tester(curve, tv);
        });
        it(name + " inv", async () => {
            const tv = buildTestVector1(curve, "inv");
            await tester(curve, tv);
        });
        it(name + " div", async () => {
            const tv = buildTestVector2(curve, "div");
            await tester(curve, tv);
        });
        it(name + " shl", async () => {
            const tv = buildTestVector2(curve, "shl");
            await tester(curve, tv);
        });
        it(name + " shr", async () => {
            const tv = buildTestVector2(curve, "shr");
            await tester(curve, tv);
        });
        it(name + " band", async () => {
            const tv = buildTestVector2(curve, "band");
            await tester(curve, tv);
        });
        it(name + " bor", async () => {
            const tv = buildTestVector2(curve, "bor");
            await tester(curve, tv);
        });
        it(name + " bxor", async () => {
            const tv = buildTestVector2(curve, "bxor");
            await tester(curve, tv);
        });
        it(name + " bnot", async () => {
            const tv = buildTestVector1(curve, "bnot");
            await tester(curve, tv);
        });

    }

    generateTest(bn128r, "gl");
    generateTest(bn128r, "bn128");
});

function buildTestVector2(p, op) {
    const F = new ZqField(p);
    const tv = [];
    const nums = getCriticalNumbers(p, 2);

    const excludeZero = ["div", "mod", "idiv"].indexOf(op) >= 0;

    for (let i=0; i<nums.length; i++) {
        for (let j=0; j<nums.length; j++) {
            if ((excludeZero)&&(nums[j][0].isZero())) continue;
            tv.push([
                [nums[i][1], nums[j][1], op],
                F[op](nums[i][0], nums[j][0])
            ]);
        }
    }

    return tv;
}

function buildTestVector1(p, op) {
    const F = new ZqField(p);
    const tv = [];
    const nums = getCriticalNumbers(p, 2);

    const excludeZero = ["inv"].indexOf(op) >= 0;

    for (let i=0; i<nums.length; i++) {
        if ((excludeZero)&&(nums[i][0].isZero())) continue;
        tv.push([
            [nums[i][1], op],
            F[op](nums[i][0])
        ]);
    }

    return tv;
}

function getCriticalNumbers(p, lim) {
    const numbers = [];

    addFrontier(0);
    addFrontier(bigInt(32));
    addFrontier(bigInt(64));
    addFrontier(bigInt(p.bitLength()));
    addFrontier(bigInt.one.shiftLeft(31));
    addFrontier(p.minus(bigInt.one.shiftLeft(31)));
    addFrontier(bigInt.one.shiftLeft(32));
    addFrontier(p.minus(bigInt.one.shiftLeft(32)));
    addFrontier(bigInt.one.shiftLeft(63));
    addFrontier(p.minus(bigInt.one.shiftLeft(63)));
    addFrontier(bigInt.one.shiftLeft(64));
    addFrontier(p.minus(bigInt.one.shiftLeft(64)));
    addFrontier(bigInt.one.shiftLeft(p.bitLength()-1));
    addFrontier(p.shiftRight(1));

    function addFrontier(f) {
        for (let i=-lim; i<=lim; i++) {
            let n = bigInt(f).add(bigInt(i));
            n = n.mod(p);
            if (n.isNegative()) n = p.add(n);
            addNumber(n);
        }
    }

    return numbers;

    function addNumber(n) {
        if (n.lt(bigInt("80000000", 16)) ) {
            addShortPositive(n);
            addShortMontgomeryPositive(n);
        }
        if (n.geq(p.minus(bigInt("80000000", 16))) ) {
            addShortNegative(n);
            addShortMontgomeryNegative(n);
        }
        addLongNormal(n);
        addLongMontgomery(n);

        function addShortPositive(a) {
            numbers.push([a, "0x"+a.toString(16)]);
        }

        function addShortMontgomeryPositive(a) {
            let S = "0x" + bigInt("40", 16).shiftLeft(56).add(a).toString(16);
            S = S + "," + getLongString(toMontgomery(a));
            numbers.push([a, S]);
        }

        function addShortNegative(a) {
            const b = bigInt("80000000", 16 ).add(a.minus(  p.minus(bigInt("80000000", 16 ))));
            numbers.push([a, "0x"+b.toString(16)]);
        }

        function addShortMontgomeryNegative(a) {
            const b = bigInt("80000000", 16 ).add(a.minus(  p.minus(bigInt("80000000", 16 ))));
            let S = "0x" + bigInt("40", 16).shiftLeft(56).add(b).toString(16);
            S = S + "," + getLongString(toMontgomery(a));
            numbers.push([a, S]);
        }

        function addLongNormal(a) {
            let S = "0x" + bigInt("80", 16).shiftLeft(56).toString(16);
            S = S + "," + getLongString(a);
            numbers.push([a, S]);
        }


        function addLongMontgomery(a) {

            let S = "0x" + bigInt("C0", 16).shiftLeft(56).toString(16);
            S = S + "," + getLongString(toMontgomery(a));
            numbers.push([a, S]);
        }

        function getLongString(a) {
            if (a.isZero()) {
                return "0x0";
            }
            let r = a;
            let S = "";
            while (!r.isZero()) {
                if (S!= "") S = S+",";
                S += "0x" + r.and(bigInt("FFFFFFFFFFFFFFFF", 16)).toString(16);
                r = r.shiftRight(64);
            }
            return S;
        }

        function toMontgomery(a) {
            const n64 = Math.floor((p.bitLength() - 1) / 64)+1;
            const R = bigInt.one.shiftLeft(n64*64);
            return a.times(R).mod(p);
        }

    }
}

