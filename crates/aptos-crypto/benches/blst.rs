// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blst::{blst_p1, blst_p1_add, blst_p1_affine, blst_p1_mult, blst_p2, blst_p2_affine};
use criterion::{BenchmarkId, Criterion, Throughput};
use rand::{distributions, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::ops::MulAssign;
use ark_std::UniformRand;
use aptos_crypto::{msm_all_bench_cases, serialize, rand};
use ark_std::test_rng;
use ark_serialize::CanonicalSerialize;

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct TestAptosCrypto(String);

fn random_bytes(len: usize) -> Vec<u8> {
    thread_rng()
        .sample_iter(&distributions::Standard)
        .take(len)
        .collect()
}

fn random_p1() -> blst_p1 {
    let msg = random_bytes(64);
    let dst = random_bytes(64);
    let aug = random_bytes(64);
    let mut point = blst_p1::default();
    unsafe {
        blst::blst_hash_to_g1(
            &mut point,
            msg.as_ptr(),
            msg.len(),
            dst.as_ptr(),
            dst.len(),
            aug.as_ptr(),
            aug.len(),
        );
    }
    point
}

fn random_p1_affine() -> blst_p1_affine {
    let p = random_p1();
    let mut p_affine = blst_p1_affine::default();
    unsafe {
        blst::blst_p1_to_affine(&mut p_affine, &p);
    }
    p_affine
}

fn random_p2() -> blst_p2 {
    let msg = random_bytes(64);
    let dst = random_bytes(64);
    let aug = random_bytes(64);
    let mut point = blst_p2::default();
    unsafe {
        blst::blst_hash_to_g2(
            &mut point,
            msg.as_ptr(),
            msg.len(),
            dst.as_ptr(),
            dst.len(),
            aug.as_ptr(),
            aug.len(),
        );
    }
    point
}

fn random_p2_affine() -> blst_p2_affine {
    let p = random_p2();
    let mut p_affine = blst_p2_affine::default();
    unsafe {
        blst::blst_p2_to_affine(&mut p_affine, &p);
    }
    p_affine
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("blst");

    group.throughput(Throughput::Elements(1));

    group.bench_function("g1_affine_serialize_comp", move |b| {
        b.iter_with_setup(random_p1_affine, |p_affine| {
            let mut out = vec![0_u8; 48];
            unsafe {
                blst::blst_p1_affine_compress(out.as_mut_ptr(), &p_affine);
            }
        })
    });

    group.bench_function("g1_affine_serialize_uncomp", move |b| {
        b.iter_with_setup(random_p1_affine, |p_affine| {
            let mut out = vec![0_u8; 96];
            unsafe {
                blst::blst_p1_affine_serialize(out.as_mut_ptr(), &p_affine);
            }
        })
    });

    group.bench_function("g1_proj_serialize", move |b| {
        b.iter_with_setup(random_p1, |p| {
            let mut out = vec![0_u8; 144];
            unsafe {
                blst::blst_p1_serialize(out.as_mut_ptr(), &p);
            }
        })
    });

    group.bench_function("g1_proj_to_affine", move |b| {
        b.iter_with_setup(random_p1, |p| {
            let mut out = blst_p1_affine::default();
            unsafe {
                blst::blst_p1_to_affine(&mut out, &p);
            }
        })
    });

    group.bench_function("g1_affine_deserialize_uncomp_input_size_96", move |b| {
        b.iter_with_setup(
            || {
                let p_affine = random_p1_affine();
                let mut buf = vec![0_u8; 96];
                unsafe {
                    blst::blst_p1_affine_serialize(buf.as_mut_ptr(), &p_affine);
                }
                buf
            },
            |buf| {
                let mut p_affine = blst::blst_p1_affine::default();
                unsafe {
                    blst::blst_p1_deserialize(&mut p_affine, buf.as_ptr());
                }
            },
        )
    });

    group.bench_function("g1_affine_deserialize_uncomp_input_size_960", move |b| {
        b.iter_with_setup(
            || {
                let p_affine = random_p1_affine();
                let mut buf = vec![0xFF_u8; 960];
                unsafe {
                    blst::blst_p1_affine_serialize(buf.as_mut_ptr(), &p_affine);
                }
                buf
            },
            |buf| {
                let mut p_affine = blst::blst_p1_affine::default();
                unsafe {
                    blst::blst_p1_deserialize(&mut p_affine, buf.as_ptr());
                }
            },
        )
    });

    group.bench_function("g1_affine_deserialize_uncomp_input_size_9600", move |b| {
        b.iter_with_setup(
            || {
                let p_affine = random_p1_affine();
                let mut buf = vec![0xFF_u8; 9600];
                unsafe {
                    blst::blst_p1_affine_serialize(buf.as_mut_ptr(), &p_affine);
                }
                buf
            },
            |buf| {
                let mut p_affine = blst::blst_p1_affine::default();
                unsafe {
                    blst::blst_p1_deserialize(&mut p_affine, buf.as_ptr());
                }
            },
        )
    });

    group.bench_function("g1_affine_deserialize_comp", move |b| {
        b.iter_with_setup(
            || {
                let p_affine = random_p1_affine();
                let mut buf = vec![0_u8; 48];
                unsafe {
                    blst::blst_p1_affine_compress(buf.as_mut_ptr(), &p_affine);
                }
                buf
            },
            |buf| {
                let mut p_affine = blst::blst_p1_affine::default();
                unsafe {
                    blst::blst_p1_uncompress(&mut p_affine, buf.as_ptr());
                }
            },
        )
    });

    group.bench_function("g1_scalar_mul", move |b| {
        b.iter_with_setup(
            || {
                let point = random_p1();
                let scalar_bytes = random_bytes(256);
                (point, scalar_bytes)
            },
            |(point, scalar_bytes)| {
                let mut out = blst_p1::default();
                unsafe {
                    blst::blst_p1_mult(&mut out, &point, scalar_bytes.as_ptr(), 256);
                }
            },
        )
    });

    group.bench_function("g2_proj_serialize", move |b| {
        b.iter_with_setup(random_p2, |p| {
            let mut out = vec![0_u8; 288];
            unsafe {
                blst::blst_p2_serialize(out.as_mut_ptr(), &p);
            }
        })
    });

    group.bench_function("g2_proj_to_affine", move |b| {
        b.iter_with_setup(random_p2, |p| {
            let mut out = blst_p2_affine::default();
            unsafe {
                blst::blst_p2_to_affine(&mut out, &p);
            }
        })
    });

    for input_byte_length in (1..4097).step_by(45) {
        group.bench_function(
            format!("hash_{input_byte_length}_bytes_to_g1_proj").as_str(),
            move |b| {
                b.iter_with_setup(
                    || {
                        let msg = random_bytes(input_byte_length);
                        let dst = random_bytes(32);
                        let aug = random_bytes(32);
                        (msg, dst, aug)
                    },
                    |(msg, dst, aug)| {
                        let mut point = blst_p1::default();
                        unsafe {
                            blst::blst_hash_to_g1(
                                &mut point,
                                msg.as_ptr(),
                                msg.len(),
                                dst.as_ptr(),
                                dst.len(),
                                aug.as_ptr(),
                                aug.len(),
                            );
                        }
                    },
                )
            },
        );
    }

    for input_byte_length in (1..4097).step_by(45) {
        group.bench_function(
            format!("hash_{input_byte_length}_bytes_to_g2_proj").as_str(),
            move |b| {
                b.iter_with_setup(
                    || {
                        let msg = random_bytes(input_byte_length);
                        let dst = random_bytes(32);
                        let aug = random_bytes(32);
                        (msg, dst, aug)
                    },
                    |(msg, dst, aug)| {
                        let mut point = blst_p2::default();
                        unsafe {
                            blst::blst_hash_to_g2(
                                &mut point,
                                msg.as_ptr(),
                                msg.len(),
                                dst.as_ptr(),
                                dst.len(),
                                aug.as_ptr(),
                                aug.len(),
                            );
                        }
                    },
                )
            },
        );
    }

    for num_pairs in [1, 2, 4, 8] {
        group.bench_function(
            format!("{num_pairs}_pairing_product").as_str(),
            move |b| unsafe {
                b.iter_with_setup(
                    || {
                        let p1_affines: Vec<blst::blst_p1_affine> =
                            (0..num_pairs).map(|_| random_p1_affine()).collect();
                        let p2_affines: Vec<blst::blst_p2_affine> =
                            (0..num_pairs).map(|_| random_p2_affine()).collect();
                        (p1_affines, p2_affines)
                    },
                    |(affine_g1_points, affine_g2_points)| {
                        let mut tmp_product = blst::blst_fp12_one().read();
                        for (p1_affine, p2_affine) in
                            affine_g1_points.iter().zip(affine_g2_points.iter())
                        {
                            let mut tmp = blst::blst_fp12::default();
                            blst::blst_miller_loop(&mut tmp, p2_affine, p1_affine);
                            blst::blst_fp12::mul_assign(&mut tmp_product, tmp);
                        }
                        let mut finaled = blst::blst_fp12::default();
                        blst::blst_final_exp(&mut finaled, &tmp_product);
                    },
                )
            },
        );
    }

    group.bench_function("g1_proj_add", move |b| {
        b.iter_with_setup(
            || (random_p1(), random_p1()),
            |(p1, p2)| {
                let mut res = blst_p1::default();
                unsafe {
                    blst_p1_add(&mut res, &p1, &p2);
                }
            },
        )
    });

    group.bench_function("g1_proj_scalar_mul", move |b| {
        b.iter_with_setup(
            || (random_p1(), random_bytes(256)),
            |(p, k)| {
                let mut res = blst_p1::default();
                unsafe {
                    blst_p1_mult(&mut res, &p, k.as_ptr(), 256);
                }
            },
        )
    });

    for num_entries in msm_all_bench_cases() {
        group.bench_function(BenchmarkId::new("g1_msm", num_entries), move |b| {
            b.iter_with_setup(
                || {
                    let points = (0..num_entries).map(|_i| random_p1()).collect::<Vec<_>>();
                    let affine_points = blst::p1_affines::from(points.as_slice());
                    let scalars_bytes = (0..num_entries).flat_map(|_i| {
                        serialize!(rand!(ark_bls12_381::Fr), serialize_uncompressed).into_iter()
                    }).collect::<Vec<_>>();
                    assert_eq!(32*num_entries, scalars_bytes.len());
                    (affine_points, scalars_bytes)
                },
                |(affine_points, scalars_bytes)| {
                    let _actual = affine_points.mult(scalars_bytes.as_slice(), 256);
                },
            )
        });
        group.bench_function(BenchmarkId::new("g2_msm", num_entries), move |b| {
            b.iter_with_setup(
                || {
                    let points = (0..num_entries).map(|_i| random_p2()).collect::<Vec<_>>();
                    let affine_points = blst::p2_affines::from(points.as_slice());
                    let scalars_bytes = (0..num_entries).flat_map(|_i| {
                        serialize!(rand!(ark_bls12_381::Fr), serialize_uncompressed).into_iter()
                    }).collect::<Vec<_>>();
                    assert_eq!(32*num_entries, scalars_bytes.len());
                    (affine_points, scalars_bytes)
                },
                |(affine_points, scalars_bytes)| {
                    let _actual = affine_points.mult(scalars_bytes.as_slice(), 256);
                },
            )
        });
    }

    group.finish();
}

criterion_group!(
    name = blst_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(blst_benches);
