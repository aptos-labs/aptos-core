pragma circom 2.1.3;

include "circomlib/circuits/poseidon.circom";
include "circomlib/circuits/comparators.circom";

// Enforces that each scalar in the input array `in` will fit in a byte
template CheckAreBytes(numBytes) {
    signal input in[numBytes];

    for (var i = 0; i < numBytes; i++) {
        var is_byte = LessThan(9)([in[i], 256]);
        is_byte === 1;
    }
}

// Enforces that each scalar in the input array `in` will fit in a limb of size 64
template CheckAre64BitLimbs(numLimbs) {
    signal input in[numLimbs];

    for (var i = 0; i < numLimbs; i++) {
        var is_byte = LessThan(65)([in[i], 2**64]);
        is_byte === 1;
    }
}

// Hashes multiple bytes to one field element using a Poseidon hash
// We hash the length of the input as well to prevent collisions
// Currently does not work with greater than 64*31=1984 bytes
//
// Warning: `numBytes` cannot be 0.
template HashBytesToFieldWithLen(numBytes) {
    signal input in[numBytes];
    signal input len;
    signal output hash;

    CheckAreBytes(numBytes)(in);

    var num_elems = numBytes%31 == 0 ? numBytes\31 : numBytes\31 + 1; 

    signal input_packed[num_elems] <== ChunksToFieldElems(numBytes, 31, 8)(in); // Pack 31 bytes per field element

    signal input_with_len[num_elems+1];
    for (var i = 0; i < num_elems; i++) {
        input_with_len[i] <== input_packed[i];
    }
    input_with_len[num_elems] <== len;

    hash <== HashElemsToField(num_elems+1)(input_with_len);
}

// Hashes multiple bytes to one field element using a Poseidon hash
// We hash the length of the input as well to prevent collisions
// Currently does not work with greater than 64*31=1984 bytes
//
// Warning: `numBytes` cannot be 0.
template HashBytesToField(numBytes) {
    signal input in[numBytes];
    signal output hash;

    CheckAreBytes(numBytes)(in);

    var num_elems = numBytes%31 == 0 ? numBytes\31 : numBytes\31 + 1; 

    signal input_packed[num_elems] <== ChunksToFieldElems(numBytes, 31, 8)(in); // Pack 31 bytes per field element

    hash <== HashElemsToField(num_elems)(input_packed);
}

// Hashes multiple field elements to one using Poseidon. Works up to 64 input elements
template HashElemsToField(numElems) {
    signal input in[numElems];
    signal output hash;

    if (numElems <= 16) { 
        hash <== Poseidon(numElems)(in);
    } else if (numElems <= 32) {
        signal inputs_one[16];
        for (var i = 0; i < 16; i++) {
            inputs_one[i] <== in[i];
        }
        signal inputs_two[numElems-16];
        for (var i = 16; i < numElems; i++) {
            inputs_two[i-16] <== in[i];
        }
        signal h1 <== Poseidon(16)(inputs_one);
        signal h2 <== Poseidon(numElems-16)(inputs_two);
        hash <== Poseidon(2)([h1, h2]);
    } else if (numElems <= 48) {
        signal inputs_one[16];
        for (var i = 0; i < 16; i++) {
            inputs_one[i] <== in[i];
        }
        signal inputs_two[16];
        for (var i = 16; i < 32; i++) {
            inputs_two[i-16] <== in[i];
        }
        signal inputs_three[numElems-32];
        for (var i = 32; i < numElems; i++) {
            inputs_three[i-32] <== in[i];
        }
        signal h1 <== Poseidon(16)(inputs_one);
        signal h2 <== Poseidon(16)(inputs_two);
        signal h3 <== Poseidon(numElems-32)(inputs_three);
        hash <== Poseidon(3)([h1, h2, h3]); 
    } else if (numElems <= 64) {
        signal inputs_one[16];
        for (var i = 0; i < 16; i++) {
            inputs_one[i] <== in[i];
        }
        signal inputs_two[16];
        for (var i = 16; i < 32; i++) {
            inputs_two[i-16] <== in[i];
        }
        signal inputs_three[16];
        for (var i = 32; i < 48; i++) {
            inputs_three[i-32] <== in[i];
        }
        signal inputs_four[numElems-48];
        for (var i = 48; i < numElems; i++) {
            inputs_four[i-48] <== in[i];
        }
        signal h1 <== Poseidon(16)(inputs_one);
        signal h2 <== Poseidon(16)(inputs_two);
        signal h3 <== Poseidon(16)(inputs_three);
        signal h4 <== Poseidon(numElems-48)(inputs_four);
        hash <== Poseidon(4)([h1, h2, h3, h4]);  
    } else {
        1 === 0;
    }

}

// Hashes multiple 64 bit limbs to one field element using a Poseidon hash
// We hash the length of the input as well to avoid collisions
//
// Warning: `numLimbs` cannot be 0.
template Hash64BitLimbsToFieldWithLen(numLimbs) {
    signal input in[numLimbs];
    signal input len;

    CheckAre64BitLimbs(numLimbs)(in);

    var num_elems = numLimbs%3 == 0 ? numLimbs\3 : numLimbs\3 + 1; 

    signal input_packed[num_elems] <== ChunksToFieldElems(numLimbs, 3, 64)(in); // Pack 3 64-bit limbs per field element

    signal input_with_len[num_elems+1];
    for (var i = 0; i < num_elems; i++) {
        input_with_len[i] <== input_packed[i];
    }
    input_with_len[num_elems] <== len;

    signal output hash <== Poseidon(num_elems+1)(input_with_len);
}

// Inspired by `Bits2Num` in circomlib. Packs chunks of bits into a single field element
template ChunksToFieldElem(numChunks, bitsPerChunk) {
    signal input in[numChunks];
 
    signal output out;
    var lc1 = in[0];

    var e2 = 2**bitsPerChunk;
    for (var i = 1; i<numChunks; i++) {
        lc1 += in[i] * e2;
        e2 = e2 * (2**bitsPerChunk);
    }

    lc1 ==> out;
}

// Packs chunks into multiple field elements
// `inputLen` cannot be 0.
template ChunksToFieldElems(inputLen, chunksPerFieldElem, bitsPerChunk) {
    signal input in[inputLen];
    var num_elems = inputLen%chunksPerFieldElem == 0 ? inputLen \ chunksPerFieldElem : (inputLen\chunksPerFieldElem) + 1; // '\' is the quotient operation - we add 1 if there are extra bits past the full chunks
    signal output elems[num_elems];
    component chunks_2_field[num_elems]; 
    for (var i = 0; i < num_elems-1; i++) {
        chunks_2_field[i] = ChunksToFieldElem(chunksPerFieldElem, bitsPerChunk); // assign circuit component
    }

    // If we have an extra chunk that isn't full of bits, we truncate the Bits2NumBigEndian component size for that chunk. This is equivalent to 0 padding the end of the array
    var num_extra_chunks = inputLen % chunksPerFieldElem;
    if (num_extra_chunks == 0) {
        num_extra_chunks = chunksPerFieldElem; // The last field element is full
        chunks_2_field[num_elems-1] = ChunksToFieldElem(chunksPerFieldElem, bitsPerChunk);
    } else {
        chunks_2_field[num_elems-1] = ChunksToFieldElem(num_extra_chunks, bitsPerChunk);
    }

    // Assign all but the last field element
    for (var i = 0; i < num_elems-1; i++) {
        for (var j = 0; j < chunksPerFieldElem; j++) {
            var index = (i * chunksPerFieldElem) + j;
            chunks_2_field[i].in[j] <== in[index];
        }
        chunks_2_field[i].out ==> elems[i];
    }

    // Assign the last field element
    var i = num_elems-1;
    for (var j = 0; j < num_extra_chunks; j++) {
        var index = (i*chunksPerFieldElem) + j;
        chunks_2_field[num_elems-1].in[j] <== in[index];
    }
    chunks_2_field[i].out ==> elems[i];
}
