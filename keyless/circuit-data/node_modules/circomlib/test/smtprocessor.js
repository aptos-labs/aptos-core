const chai = require("chai");
const path = require("path");
const wasm_tester = require("circom_tester").wasm;
const F1Field = require("ffjavascript").F1Field;
const Scalar = require("ffjavascript").Scalar;

const newMemEmptyTrie = require("circomlibjs").newMemEmptyTrie;

const assert = chai.assert;

function print(circuit, w, s) {
    console.log(s + ": " + w[circuit.getSignalIdx(s)]);
}

async function testInsert(tree, _key, _value, circuit ) {
    const key = tree.F.e(_key);
    const value = tree.F.e(_value)

    const res = await tree.insert(key,value);
    let siblings = res.siblings;
    for (let i=0; i<siblings.length; i++) siblings[i] = tree.F.toObject(siblings[i]);
    while (siblings.length<10) siblings.push(0);

    const w = await circuit.calculateWitness({
        fnc: [1,0],
        oldRoot: tree.F.toObject(res.oldRoot),
        siblings: siblings,
        oldKey: res.isOld0 ? 0 : tree.F.toObject(res.oldKey),
        oldValue: res.isOld0 ? 0 : tree.F.toObject(res.oldValue),
        isOld0: res.isOld0 ? 1 : 0,
        newKey: tree.F.toObject(key),
        newValue: tree.F.toObject(value)
    }, true);

    await circuit.checkConstraints(w);

    await circuit.assertOut(w, {newRoot: tree.F.toObject(res.newRoot)});

}

async function testDelete(tree, _key, circuit) {
    const key = tree.F.e(_key);
    const res = await tree.delete(key);
    let siblings = res.siblings;
    for (let i=0; i<siblings.length; i++) siblings[i] = tree.F.toObject(siblings[i]);
    while (siblings.length<10) siblings.push(0);

    const w = await circuit.calculateWitness({
        fnc: [1,1],
        oldRoot: tree.F.toObject(res.oldRoot),
        siblings: siblings,
        oldKey: res.isOld0 ? 0 : tree.F.toObject(res.oldKey),
        oldValue: res.isOld0 ? 0 : tree.F.toObject(res.oldValue),
        isOld0: res.isOld0 ? 1 : 0,
        newKey: tree.F.toObject(res.delKey),
        newValue: tree.F.toObject(res.delValue)
    }, true);

    await circuit.checkConstraints(w);

    await circuit.assertOut(w, {newRoot: tree.F.toObject(res.newRoot)});
}

async function testUpdate(tree, _key, _newValue, circuit) {
    const key = tree.F.e(_key);
    const newValue = tree.F.e(_newValue);
    const res = await tree.update(key, newValue);
    let siblings = res.siblings;
    for (let i=0; i<siblings.length; i++) siblings[i] = tree.F.toObject(siblings[i]);
    while (siblings.length<10) siblings.push(0);

    const w = await circuit.calculateWitness({
        fnc: [0,1],
        oldRoot: tree.F.toObject(res.oldRoot),
        siblings: siblings,
        oldKey: tree.F.toObject(res.oldKey),
        oldValue: tree.F.toObject(res.oldValue),
        isOld0: 0,
        newKey: tree.F.toObject(res.newKey),
        newValue: tree.F.toObject(res.newValue)
    });

    await circuit.checkConstraints(w);

    await circuit.assertOut(w, {newRoot: tree.F.toObject(res.newRoot)});
}


describe("SMT Processor test", function () {
    let circuit;
    let tree;
    let Fr;

    this.timeout(1000000000);

    before( async () => {
        circuit = await wasm_tester(path.join(__dirname, "circuits", "smtprocessor10_test.circom"));
        await circuit.loadSymbols();

        tree = await newMemEmptyTrie();
        Fr = tree.F;
    });

    it("Should verify an insert to an empty tree", async () => {
        const key = Fr.e(111);
        const value = Fr.e(222);

        await testInsert(tree, key, value, circuit);
    });

    it("It should add another element", async () => {
        const key = Fr.e(333);
        const value = Fr.e(444);

        await testInsert(tree, key, value, circuit);
    });

    it("Should remove an element", async () => {
        await testDelete(tree, 111, circuit);
        await testDelete(tree, 333, circuit);
    });

    it("Should test convination of adding and removing 3 elements", async () => {
        const keys = [Fr.e(8), Fr.e(9), Fr.e(32)];
        const values = [Fr.e(88), Fr.e(99), Fr.e(3232)];
        const tree1 = await newMemEmptyTrie();
        const tree2 = await newMemEmptyTrie();
        const tree3 = await newMemEmptyTrie();
        const tree4 = await newMemEmptyTrie();
        const tree5 = await newMemEmptyTrie();
        const tree6 = await newMemEmptyTrie();

        await testInsert(tree1,keys[0],values[0], circuit);
        await testInsert(tree1,keys[1],values[1], circuit);
        await testInsert(tree1,keys[2],values[2], circuit);

        await testInsert(tree2,keys[0],values[0], circuit);
        await testInsert(tree2,keys[2],values[2], circuit);
        await testInsert(tree2,keys[1],values[1], circuit);

        await testInsert(tree3,keys[1],values[1], circuit);
        await testInsert(tree3,keys[0],values[0], circuit);
        await testInsert(tree3,keys[2],values[2], circuit);

        await testInsert(tree4,keys[1],values[1], circuit);
        await testInsert(tree4,keys[2],values[2], circuit);
        await testInsert(tree4,keys[0],values[0], circuit);

        await testInsert(tree5,keys[2],values[2], circuit);
        await testInsert(tree5,keys[0],values[0], circuit);
        await testInsert(tree5,keys[1],values[1], circuit);

        await testInsert(tree6,keys[2],values[2], circuit);
        await testInsert(tree6,keys[1],values[1], circuit);
        await testInsert(tree6,keys[0],values[0], circuit);


        await testDelete(tree1, keys[0], circuit);
        await testDelete(tree1, keys[1], circuit);
        await testDelete(tree2, keys[1], circuit);
        await testDelete(tree2, keys[0], circuit);

        await testDelete(tree3, keys[0], circuit);
        await testDelete(tree3, keys[2], circuit);
        await testDelete(tree4, keys[2], circuit);
        await testDelete(tree4, keys[0], circuit);


        await testDelete(tree5, keys[1], circuit);
        await testDelete(tree5, keys[2], circuit);
        await testDelete(tree6, keys[2], circuit);
        await testDelete(tree6, keys[1], circuit);

        await testDelete(tree1, keys[2], circuit);
        await testDelete(tree2, keys[2], circuit);
        await testDelete(tree3, keys[1], circuit);
        await testDelete(tree4, keys[1], circuit);
        await testDelete(tree5, keys[0], circuit);
        await testDelete(tree6, keys[0], circuit);
    });

    it("Should match a NOp with random vals", async () => {
        let siblings = [];
        while (siblings.length<10) siblings.push(88);
        const w = await circuit.calculateWitness({
            fnc: [0,0],
            oldRoot: 11,
            siblings: siblings,
            oldKey: 33,
            oldValue: 44,
            isOld0: 55,
            newKey: 66,
            newValue: 77
        });

        const root1 = Fr.e(w[circuit.symbols["main.oldRoot"].varIdx]);
        const root2 = Fr.e(w[circuit.symbols["main.newRoot"].varIdx]);

        await circuit.checkConstraints(w);

        assert(Fr.eq(root1, root2));
    });
    it("Should update an element", async () => {
        const tree1 = await newMemEmptyTrie();
        const tree2 = await newMemEmptyTrie();

        await testInsert(tree1,8,88, circuit);
        await testInsert(tree1,9,99, circuit);
        await testInsert(tree1,32,3232, circuit);

        await testInsert(tree2,8,888, circuit);
        await testInsert(tree2,9,999, circuit);
        await testInsert(tree2,32,323232, circuit);

        await testUpdate(tree1, 8, 888, circuit);
        await testUpdate(tree1, 9, 999, circuit);
        await testUpdate(tree1, 32, 323232, circuit);
    });
});
