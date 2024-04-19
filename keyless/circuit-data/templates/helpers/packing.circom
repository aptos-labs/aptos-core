pragma circom 2.1.3;

include "helpers/arrays.circom";

// Based on `Num2Bits` in circomlib
template Num2BitsBE(n) {
    signal input in;
    signal output out[n];
    var lc1 = 0;
   
    var e2 = 1;
    for (var i = 0; i < n; i++) {
        var idx = (n - 1) - i;
        out[idx] <-- (in >> i) & 1;
        out[idx] * (out[idx] - 1 ) === 0;
        lc1 += out[idx] * e2;
        e2 = e2 + e2;
    }
    lc1 === in;
}

// Converts a bit array into a big endian integer
// Inspired by Bits2Num in https://github.com/iden3/circomlib/blob/master/circuits/bitify.circom
template Bits2NumBigEndian(n) { 
    signal input in[n];
    signal output out;
    var lc1=0;
    
    var e2 = 1;
    for (var i = 0; i < n; i++) {
        var index = n-1-i;
        lc1 += in[index] * e2;
        e2 = e2 + e2;
    }
    lc1 ==> out;
}


// Converts byte array `in` into a bit array
template BytesToBits(inputLen) {
    signal input in[inputLen];
    var byte_len = 8;
    signal output bits[byte_len*inputLen];
    component num2bits[inputLen];
    for (var i = 0; i < inputLen; i++) {
        num2bits[i] = Num2BitsBE(byte_len);
        num2bits[i].in <== in[i];
        for (var j = 0; j < byte_len; j++) {
            var index = (i*byte_len)+j;
            num2bits[i].out[j] ==> bits[index];
        }
    }
}

// Assumes decimals fit into a single 254 bit value when using BN254
template ASCIIDecimalsToFieldElem(inputLen) {
    signal input in[inputLen];
    signal output out;

    var lc1 = in[0];
     
    var e2 = 10;
    for (var i = 1; i<inputLen; i++) {
        lc1 += in[i] * e2;
        e2 = e2 * 10;
    }

    lc1 ==> out;
}

// Converts bit array 'in' into an array of field elements of size `bitsPerFieldElem` each
template BitsToFieldElems(inputLen, bitsPerFieldElem) {
    signal input in[inputLen];
    var num_elems = inputLen%bitsPerFieldElem == 0 ? inputLen \ bitsPerFieldElem : (inputLen\bitsPerFieldElem) + 1; // '\' is the quotient operation - we add 1 if there are extra bits past the full bytes
    signal output elems[num_elems];
    component bits_2_num_be[num_elems]; 
    for (var i = 0; i < num_elems-1; i++) {
        bits_2_num_be[i] = Bits2NumBigEndian(bitsPerFieldElem); // assign circuit component
    }

    // If we have an extra byte that isn't full of bits, we truncate the Bits2NumBigEndian component size for that byte. This is equivalent to 0 padding the end of the array
    var num_extra_bits = inputLen % bitsPerFieldElem;
    if (num_extra_bits == 0) {
        num_extra_bits = bitsPerFieldElem; // The last field element is full
        bits_2_num_be[num_elems-1] = Bits2NumBigEndian(bitsPerFieldElem);
    } else {
        bits_2_num_be[num_elems-1] = Bits2NumBigEndian(num_extra_bits);
    }

    // Assign all but the last field element
    for (var i = 0; i < num_elems-1; i++) {
        for (var j = 0; j < bitsPerFieldElem; j++) {
            var index = (i * bitsPerFieldElem) + j;
            bits_2_num_be[i].in[j] <== in[index];
        }
        bits_2_num_be[i].out ==> elems[i];
    }

    // Assign the last field element
    for (var j = 0; j < num_extra_bits; j++) {
        var i = num_elems-1;
        var index = (i*bitsPerFieldElem) + j;
        bits_2_num_be[num_elems-1].in[j] <== in[index];
    }
    bits_2_num_be[num_elems-1].out ==> elems[num_elems-1];
}

// Assumes `in` is a little endian bit array where `48` corresponds to `0` and `49` corresponds to `1`
// Packs a maximum of `maxBitsPerFieldElem` bits per field element, actually packs `to_pack_len`
// TODO: Not tested for more than one field element. We only ever use it to pack one field element anyways
template AsciiBitsLEToFieldElems(inputLen, maxBitsPerFieldElem) {
    signal input in[inputLen];
    signal input to_pack_len;
    signal selector_bits[inputLen] <== ArraySelector(inputLen)(0, to_pack_len);
    var num_elems = inputLen%maxBitsPerFieldElem == 0 ? inputLen \ maxBitsPerFieldElem : (inputLen\maxBitsPerFieldElem) + 1; // '\' is the quotient operation - we add 1 if there are extra bits past the full bytes
    signal output elems[num_elems];
    component bits_2_num_be[num_elems]; 
    for (var i = 0; i < num_elems-1; i++) {
        bits_2_num_be[i] = Bits2Num(maxBitsPerFieldElem); // assign circuit component
    }

    // If we have an extra byte that isn't full of bits, we truncate the Bits2NumBigEndian component size for that byte. This is equivalent to 0 padding the end of the array
    var num_extra_bits = inputLen % maxBitsPerFieldElem;
    if (num_extra_bits == 0) {
        num_extra_bits = maxBitsPerFieldElem; // The last field element is full
        bits_2_num_be[num_elems-1] = Bits2Num(maxBitsPerFieldElem);
    } else {
        bits_2_num_be[num_elems-1] = Bits2Num(num_extra_bits);
    }

    // Assign all but the last field element
    for (var i = 0; i < num_elems-1; i++) {
        for (var j = 0; j < maxBitsPerFieldElem; j++) {
            var index = (i * maxBitsPerFieldElem) + j;
            bits_2_num_be[i].in[j] <== in[index]-48 * selector_bits[index];
        }
        bits_2_num_be[i].out ==> elems[i];
    }

    // Assign the last field element
    var i = num_elems-1;
    for (var j = 0; j < num_extra_bits; j++) {
        var index = (i*maxBitsPerFieldElem) + j;
        bits_2_num_be[num_elems-1].in[j] <== in[index]-48 * selector_bits[index];
    }
    bits_2_num_be[i].out ==> elems[i];
}
