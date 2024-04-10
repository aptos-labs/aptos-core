pragma circom 2.0.2;

// File copied from https://github.com/doubleblind-xyz/circom-rsa/blob/master/circuits/bigint_func.circom

function isNegative(x) {
    // half babyjubjub field size
    return x > 10944121435919637611123202872628637544274182200208017171849102093287904247808 ? 1 : 0;
}

function div_ceil(m, n) {
    var ret = 0;
    if (m % n == 0) {
        ret = m \ n;
    } else {
        ret = m \ n + 1;
    }
    return ret;
}

function log_ceil(n) {
   var n_temp = n;
   for (var i = 0; i < 254; i++) {
       if (n_temp == 0) {
          return i;
       }
       n_temp = n_temp \ 2;
   }
   return 254;
}

function SplitFn(in, n, m) {
    return [in % (1 << n), (in \ (1 << n)) % (1 << m)];
}

function SplitThreeFn(in, n, m, k) {
    return [in % (1 << n), (in \ (1 << n)) % (1 << m), (in \ (1 << n + m)) % (1 << k)];
}

// m bits per overflowed register (values are potentially negative)
// n bits per properly-sized register
// in has k registers
// out has k + ceil(m/n) - 1 + 1 registers. highest-order potentially negative,
// all others are positive
// - 1 since the last register is included in the last ceil(m/n) array
// + 1 since the carries from previous registers could push you over
function getProperRepresentation(m, n, k, in) {
    var ceilMN = div_ceil(m, n);

    var out[100]; // should be out[k + ceilMN]
    assert(k + ceilMN < 100);
    for (var i = 0; i < k; i++) {
        out[i] = in[i];
    }
    for (var i = k; i < 100; i++) {
        out[i] = 0;
    }
    assert(n <= m);
    for (var i = 0; i+1 < k + ceilMN; i++) {
        assert((1 << m) >= out[i] && out[i] >= -(1 << m));
        var shifted_val = out[i] + (1 << m);
        assert(0 <= shifted_val && shifted_val <= (1 << (m+1)));
        out[i] = shifted_val & ((1 << n) - 1);
        out[i+1] += (shifted_val >> n) - (1 << (m - n));
    }

    return out;
}

// Evaluate polynomial a at point x
function poly_eval(len, a, x) {
    var v = 0;
    for (var i = 0; i < len; i++) {
        v += a[i] * (x ** i);
    }
    return v;
}

// Interpolate a degree len-1 polynomial given its evaluations at 0..len-1
function poly_interp(len, v) {
    assert(len <= 200);
    var out[200];
    for (var i = 0; i < len; i++) {
        out[i] = 0;
    }

    // Product_{i=0..len-1} (x-i)
    var full_poly[201];
    full_poly[0] = 1;
    for (var i = 0; i < len; i++) {
        full_poly[i+1] = 0;
        for (var j = i; j >= 0; j--) {
            full_poly[j+1] += full_poly[j];
            full_poly[j] *= -i;
        }
    }

    for (var i = 0; i < len; i++) {
        var cur_v = 1;
        for (var j = 0; j < len; j++) {
            if (i == j) {
                // do nothing
            } else {
                cur_v *= i-j;
            }
        }
        cur_v = v[i] / cur_v;

        var cur_rem = full_poly[len];
        for (var j = len-1; j >= 0; j--) {
            out[j] += cur_v * cur_rem;
            cur_rem = full_poly[j] + i * cur_rem;
        }
        assert(cur_rem == 0);
    }

    return out;
}

// 1 if true, 0 if false
function long_gt(n, k, a, b) {
    for (var i = k - 1; i >= 0; i--) {
        if (a[i] > b[i]) {
            return 1;
        }
        if (a[i] < b[i]) {
            return 0;
        }
    }
    return 0;
}

// n bits per register
// a has k registers
// b has k registers
// a >= b
function long_sub(n, k, a, b) {
    var diff[100];
    var borrow[100];
    for (var i = 0; i < k; i++) {
        if (i == 0) {
           if (a[i] >= b[i]) {
               diff[i] = a[i] - b[i];
               borrow[i] = 0;
            } else {
               diff[i] = a[i] - b[i] + (1 << n);
               borrow[i] = 1;
            }
        } else {
            if (a[i] >= b[i] + borrow[i - 1]) {
               diff[i] = a[i] - b[i] - borrow[i - 1];
               borrow[i] = 0;
            } else {
               diff[i] = (1 << n) + a[i] - b[i] - borrow[i - 1];
               borrow[i] = 1;
            }
        }
    }
    return diff;
}

// a is a n-bit scalar
// b has k registers
function long_scalar_mult(n, k, a, b) {
    var out[100];
    for (var i = 0; i < 100; i++) {
        out[i] = 0;
    }
    for (var i = 0; i < k; i++) {
        var temp = out[i] + (a * b[i]);
        out[i] = temp % (1 << n);
        out[i + 1] = out[i + 1] + temp \ (1 << n);
    }
    return out;
}


// n bits per register
// a has k + m registers
// b has k registers
// out[0] has length m + 1 -- quotient
// out[1] has length k -- remainder
// implements algorithm of https://people.eecs.berkeley.edu/~fateman/282/F%20Wright%20notes/week4.pdf
function long_div(n, k, m, a, b){
    var out[2][100];
    m += k;
    while (b[k-1] == 0) {
        out[1][k] = 0;
        k--;
        assert(k > 0);
    }
    m -= k;

    var remainder[200];
    for (var i = 0; i < m + k; i++) {
        remainder[i] = a[i];
    }

    var mult[200];
    var dividend[200];
    for (var i = m; i >= 0; i--) {
        if (i == m) {
            dividend[k] = 0;
            for (var j = k - 1; j >= 0; j--) {
                dividend[j] = remainder[j + m];
            }
        } else {
            for (var j = k; j >= 0; j--) {
                dividend[j] = remainder[j + i];
            }
        }

        out[0][i] = short_div(n, k, dividend, b);

        var mult_shift[100] = long_scalar_mult(n, k, out[0][i], b);
        var subtrahend[200];
        for (var j = 0; j < m + k; j++) {
            subtrahend[j] = 0;
        }
        for (var j = 0; j <= k; j++) {
            if (i + j < m + k) {
               subtrahend[i + j] = mult_shift[j];
            }
        }
        remainder = long_sub(n, m + k, remainder, subtrahend);
    }
    for (var i = 0; i < k; i++) {
        out[1][i] = remainder[i];
    }
    out[1][k] = 0;

    return out;
}

// n bits per register
// a has k + 1 registers
// b has k registers
// assumes leading digit of b is at least 2 ** (n - 1)
// 0 <= a < (2**n) * b
function short_div_norm(n, k, a, b) {
   var qhat = (a[k] * (1 << n) + a[k - 1]) \ b[k - 1];
   if (qhat > (1 << n) - 1) {
      qhat = (1 << n) - 1;
   }

   var mult[100] = long_scalar_mult(n, k, qhat, b);
   if (long_gt(n, k + 1, mult, a) == 1) {
      mult = long_sub(n, k + 1, mult, b);
      if (long_gt(n, k + 1, mult, a) == 1) {
         return qhat - 2;
      } else {
         return qhat - 1;
      }
   } else {
       return qhat;
   }
}

// n bits per register
// a has k + 1 registers
// b has k registers
// assumes leading digit of b is non-zero
// 0 <= a < (2**n) * b
function short_div(n, k, a, b) {
   var scale = (1 << n) \ (1 + b[k - 1]);

   // k + 2 registers now
   var norm_a[200] = long_scalar_mult(n, k + 1, scale, a);
   // k + 1 registers now
   var norm_b[200] = long_scalar_mult(n, k, scale, b);

   var ret;
   if (norm_b[k] != 0) {
       ret = short_div_norm(n, k + 1, norm_a, norm_b);
   } else {
       ret = short_div_norm(n, k, norm_a, norm_b);
   }
   return ret;
}

// n bits per register
// a and b both have k registers
// out[0] has length 2 * k
// adapted from BigMulShortLong and LongToShortNoEndCarry2 witness computation
function prod(n, k, a, b) {
    // first compute the intermediate values. taken from BigMulShortLong
    var prod_val[100]; // length is 2 * k - 1
    for (var i = 0; i < 2 * k - 1; i++) {
        prod_val[i] = 0;
        if (i < k) {
            for (var a_idx = 0; a_idx <= i; a_idx++) {
                prod_val[i] = prod_val[i] + a[a_idx] * b[i - a_idx];
            }
        } else {
            for (var a_idx = i - k + 1; a_idx < k; a_idx++) {
                prod_val[i] = prod_val[i] + a[a_idx] * b[i - a_idx];
            }
        }
    }

    // now do a bunch of carrying to make sure registers not overflowed. taken from LongToShortNoEndCarry2
    var out[100]; // length is 2 * k

    var split[100][3]; // first dimension has length 2 * k - 1
    for (var i = 0; i < 2 * k - 1; i++) {
        split[i] = SplitThreeFn(prod_val[i], n, n, n);
    }

    var carry[100]; // length is 2 * k - 1
    carry[0] = 0;
    out[0] = split[0][0];
    if (2 * k - 1 > 1) {
        var sumAndCarry[2] = SplitFn(split[0][1] + split[1][0], n, n);
        out[1] = sumAndCarry[0];
        carry[1] = sumAndCarry[1];
    }
    if (2 * k - 1 > 2) {
        for (var i = 2; i < 2 * k - 1; i++) {
            var sumAndCarry[2] = SplitFn(split[i][0] + split[i-1][1] + split[i-2][2] + carry[i-1], n, n);
            out[i] = sumAndCarry[0];
            carry[i] = sumAndCarry[1];
        }
        out[2 * k - 1] = split[2*k-2][1] + split[2*k-3][2] + carry[2*k-2];
    }
    return out;
}

// n bits per register
// a has k registers
// p has k registers
// e has k registers
// k * n <= 500
// p is a prime
// computes a^e mod p
function mod_exp(n, k, a, p, e) {
    var eBits[500]; // length is k * n
    for (var i = 0; i < k; i++) {
        for (var j = 0; j < n; j++) {
            eBits[j + n * i] = (e[i] >> j) & 1;
        }
    }

    var out[100]; // length is k
    for (var i = 0; i < 100; i++) {
        out[i] = 0;
    }
    out[0] = 1;

    // repeated squaring
    for (var i = k * n - 1; i >= 0; i--) {
        // multiply by a if bit is 0
        if (eBits[i] == 1) {
            var temp[200]; // length 2 * k
            temp = prod(n, k, out, a);
            var temp2[2][100];
            temp2 = long_div(n, k, k, temp, p);
            out = temp2[1];
        }

        // square, unless we're at the end
        if (i > 0) {
            var temp[200]; // length 2 * k
            temp = prod(n, k, out, out);
            var temp2[2][100];
            temp2 = long_div(n, k, k, temp, p);
            out = temp2[1];
        }

    }
    return out;
}

// n bits per register
// a has k registers
// p has k registers
// k * n <= 500
// p is a prime
// if a == 0 mod p, returns 0
// else computes inv = a^(p-2) mod p
function mod_inv(n, k, a, p) {
    var isZero = 1;
    for (var i = 0; i < k; i++) {
        if (a[i] != 0) {
            isZero = 0;
        }
    }
    if (isZero == 1) {
        var ret[100];
        for (var i = 0; i < k; i++) {
            ret[i] = 0;
        }
        return ret;
    }

    var pCopy[100];
    for (var i = 0; i < 100; i++) {
        if (i < k) {
            pCopy[i] = p[i];
        } else {
            pCopy[i] = 0;
        }
    }

    var two[100];
    for (var i = 0; i < 100; i++) {
        two[i] = 0;
    }
    two[0] = 2;

    var pMinusTwo[100];
    pMinusTwo = long_sub(n, k, pCopy, two); // length k
    var out[100];
    out = mod_exp(n, k, a, pCopy, pMinusTwo);
    return out;
}

// a, b and out are all n bits k registers
function long_sub_mod_p(n, k, a, b, p){
    var gt = long_gt(n, k, a, b);
    var tmp[100];
    if(gt){
        tmp = long_sub(n, k, a, b);
    }
    else{
        tmp = long_sub(n, k, b, a);
    }
    var out[2][100];
    for(var i = k;i < 2 * k; i++){
        tmp[i] = 0;
    }
    out = long_div(n, k, k, tmp, p);
    if(gt==0){
        tmp = long_sub(n, k, p, out[1]);
    }
    return tmp;
}

// a, b, p and out are all n bits k registers
function prod_mod_p(n, k, a, b, p){
    var tmp[100];
    var result[2][100];
    tmp = prod(n, k, a, b);
    result = long_div(n, k, k, tmp, p);
    return result[1];
}
