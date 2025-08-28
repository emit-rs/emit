/*
This file comes from `rust-lang/libm`

It includes the functionality needed for exponential bucketing.
These functions have been made `const` and specialized to `f64`.
This implementation lives here so that bucketing can be done consistently
across platforms, including in `no_std` programs.

`rust-lang/libm` includes the following license:

rust-lang/libm as a whole is available for use under the MIT license:

------------------------------------------------------------------------------
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
------------------------------------------------------------------------------
*/

const BITS: u32 = 64;
const SIG_BITS: u32 = 52;
const EXP_BITS: u32 = BITS - SIG_BITS - 1;
const EXP_SAT: u32 = (1 << EXP_BITS) - 1;
const EXP_BIAS: u32 = EXP_SAT >> 1;
const EXP_MAX: i32 = EXP_BIAS as i32;
const EXP_MIN: i32 = -(EXP_MAX - 1);
const IMPLICIT_BIT: u64 = 1 << SIG_BITS;
const SIGN_MASK: u64 = 1 << (BITS - 1);
const SIG_MASK: u64 = (1 << SIG_BITS) - 1;
const EXP_MASK: u64 = !(SIGN_MASK | SIG_MASK);

mod ceil {
    use super::*;

    /**
    Find the smallest integer greater than or equal to `x`.
    */
    #[inline]
    pub const fn ceil(x: f64) -> f64 {
        let zero = 0;

        let mut ix = x.to_bits();
        let e = exp_unbiased(x);

        // If the represented value has no fractional part, no truncation is needed.
        if e >= SIG_BITS as i32 {
            return x;
        }

        if e >= 0 {
            // |x| >= 1.0
            let m = SIG_MASK >> e as u32;
            if (ix & m) == zero {
                // Portion to be masked is already zero; no adjustment needed.
                return x;
            }

            if x.is_sign_positive() {
                ix += m;
            }

            ix &= !m;
            f64::from_bits(ix)
        } else {
            if x.is_sign_negative() {
                // -1.0 < x <= -0.0; rounding up goes toward -0.0.
                -0.0
            } else if ix << 1 != zero {
                // 0.0 < x < 1.0; rounding up goes toward +1.0.
                1.0
            } else {
                // +0.0 remains unchanged
                x
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Test against https://en.cppreference.com/w/cpp/numeric/math/ceil
        #[test]
        fn spec_test() {
            let cases = [
                (0.1f64, 1.0f64),
                (-0.1, -0.0),
                (0.9, 1.0),
                (-0.9, -0.0),
                (1.1, 2.0),
                (-1.1, -1.0),
                (1.9, 2.0),
                (-1.9, -1.0),
            ];

            let roundtrip = [0.0f64, 1.0, -1.0, -0.0, f64::INFINITY, f64::NEG_INFINITY];

            for x in roundtrip {
                let val = ceil(x);
                assert_eq!(val.to_bits(), x.to_bits(), "{}", x);
            }

            for (x, res) in cases {
                let val = ceil(x);
                assert_eq!(val.to_bits(), res.to_bits(), "{}", x);
            }
        }

        #[test]
        fn sanity_check() {
            assert_eq!(ceil(1.1f64), 2.0);
            assert_eq!(ceil(2.9f64), 3.0);
        }
    }
}

pub use self::ceil::*;

mod scalbn {
    use super::*;

    /**
    Scale the exponent.

    From N3220:

    > The scalbn and scalbln functions compute `x * b^n`, where `b = FLT_RADIX` if the return type
    > of the function is a standard floating type, or `b = 10` if the return type of the function
    > is a decimal floating type. A range error occurs for some finite x, depending on n.
    >
    > [...]
    >
    > * `scalbn(±0, n)` returns `±0`.
    > * `scalbn(x, 0)` returns `x`.
    > * `scalbn(±∞, n)` returns `±∞`.
    >
    > If the calculation does not overflow or underflow, the returned value is exact and
    > independent of the current rounding direction mode.
    */
    pub const fn scalbn(mut x: f64, mut n: i32) -> f64 {
        let zero = 0;

        // Bits including the implicit bit
        let sig_total_bits = SIG_BITS + 1;

        // Maximum and minimum values when biased
        let exp_max = EXP_MAX;
        let exp_min = EXP_MIN;

        // 2 ^ Emax, maximum positive with null significand (0x1p1023 for f64)
        let f_exp_max = from_parts(false, EXP_BIAS << 1, zero);

        // 2 ^ Emin, minimum positive normal with null significand (0x1p-1022 for f64)
        let f_exp_min = from_parts(false, 1, zero);

        // 2 ^ sig_total_bits, multiplier to normalize subnormals (0x1p53 for f64)
        let f_pow_subnorm = from_parts(false, sig_total_bits + EXP_BIAS, zero);

        /*
         * The goal is to multiply `x` by a scale factor that applies `n`. However, there are cases
         * where `2^n` is not representable by `F` but the result should be, e.g. `x = 2^Emin` with
         * `n = -EMin + 2` (one out of range of 2^Emax). To get around this, reduce the magnitude of
         * the final scale operation by prescaling by the max/min power representable by `F`.
         */

        if n > exp_max {
            // Worse case positive `n`: `x`  is the minimum subnormal value, the result is `MAX`.
            // This can be reached by three scaling multiplications (two here and one final).
            debug_assert!(-exp_min + SIG_BITS as i32 + exp_max <= exp_max * 3);

            x *= f_exp_max;
            n -= exp_max;
            if n > exp_max {
                x *= f_exp_max;
                n -= exp_max;
                if n > exp_max {
                    n = exp_max;
                }
            }
        } else if n < exp_min {
            // `mul` s.t. `!(x * mul).is_subnormal() ∀ x`
            let mul = f_exp_min * f_pow_subnorm;
            let add = -exp_min - sig_total_bits as i32;

            // Worse case negative `n`: `x`  is the maximum positive value, the result is `MIN`.
            // This must be reachable by three scaling multiplications (two here and one final).
            debug_assert!(-exp_min + SIG_BITS as i32 + exp_max <= add * 2 + -exp_min);

            x *= mul;
            n += add;

            if n < exp_min {
                x *= mul;
                n += add;

                if n < exp_min {
                    n = exp_min;
                }
            }
        }

        let scale = from_parts(false, (EXP_BIAS as i32 + n) as u32, zero);
        x * scale
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Tests against N3220
        #[test]
        fn spec_test() {
            // `scalbn(±0, n)` returns `±0`.
            assert_eq!(scalbn(-0.0f64, 10).to_bits(), (-0.0f64).to_bits());
            assert_eq!(scalbn(-0.0f64, 0).to_bits(), (-0.0f64).to_bits());
            assert_eq!(scalbn(-0.0f64, -10).to_bits(), (-0.0f64).to_bits());
            assert_eq!(scalbn(0.0, 10).to_bits(), 0.0f64.to_bits());
            assert_eq!(scalbn(0.0, 0).to_bits(), 0.0f64.to_bits());
            assert_eq!(scalbn(0.0, -10).to_bits(), 0.0f64.to_bits());

            // `scalbn(x, 0)` returns `x`.
            assert_eq!(scalbn(f64::MIN, 0).to_bits(), f64::MIN.to_bits());
            assert_eq!(scalbn(f64::MAX, 0).to_bits(), f64::MAX.to_bits());
            assert_eq!(scalbn(f64::INFINITY, 0).to_bits(), f64::INFINITY.to_bits());
            assert_eq!(
                scalbn(f64::NEG_INFINITY, 0).to_bits(),
                f64::NEG_INFINITY.to_bits()
            );
            assert_eq!(scalbn(0.0, 0).to_bits(), 0.0f64.to_bits());
            assert_eq!(scalbn(-0.0f64, 0).to_bits(), (-0.0f64).to_bits());

            // `scalbn(±∞, n)` returns `±∞`.
            assert_eq!(scalbn(f64::INFINITY, 10).to_bits(), f64::INFINITY.to_bits());
            assert_eq!(
                scalbn(f64::INFINITY, -10).to_bits(),
                f64::INFINITY.to_bits()
            );
            assert_eq!(
                scalbn(f64::NEG_INFINITY, 10).to_bits(),
                f64::NEG_INFINITY.to_bits()
            );
            assert_eq!(
                scalbn(f64::NEG_INFINITY, -10).to_bits(),
                f64::NEG_INFINITY.to_bits()
            );

            // NaN should remain NaNs.
            assert!(scalbn(f64::NAN, 10).is_nan());
            assert!(scalbn(f64::NAN, 0).is_nan());
            assert!(scalbn(f64::NAN, -10).is_nan());
            assert!(scalbn(-f64::NAN, 10).is_nan());
            assert!(scalbn(-f64::NAN, 0).is_nan());
            assert!(scalbn(-f64::NAN, -10).is_nan());
        }
    }
}

pub use self::scalbn::*;

mod sqrt {
    /* SPDX-License-Identifier: MIT */
    /* origin: musl src/math/sqrt.c. Ported to generic Rust algorithm in 2025, TG. */

    use super::*;

    /**
    Generic square root algorithm.

    This routine operates around `m_u2`, a U.2 (fixed point with two integral bits) mantissa
    within the range [1, 4). A table lookup provides an initial estimate, then goldschmidt
    iterations at various widths are used to approach the real values.

    For the iterations, `r` is a U0 number that approaches `1/sqrt(m_u2)`, and `s` is a U2 number
    that approaches `sqrt(m_u2)`. Recall that m_u2 ∈ [1, 4).

    With Newton-Raphson iterations, this would be:

    - `w = r * r           w ~ 1 / m`
    - `u = 3 - m * w       u ~ 3 - m * w = 3 - m / m = 2`
    - `r = r * u / 2       r ~ r`

    (Note that the righthand column does not show anything analytically meaningful (i.e. r ~ r),
    since the value of performing one iteration is in reducing the error representable by `~`).

    Instead of Newton-Raphson iterations, Goldschmidt iterations are used to calculate
    `s = m * r`:

    - `s = m * r           s ~ m / sqrt(m)`
    - `u = 3 - s * r       u ~ 3 - (m / sqrt(m)) * (1 / sqrt(m)) = 3 - m / m = 2`
    - `r = r * u / 2       r ~ r`
    - `s = s * u / 2       s ~ s`

    The above is precise because it uses the original value `m`. There is also a faster version
    that performs fewer steps but does not use `m`:

    - `u = 3 - s * r       u ~ 3 - 1`
    - `r = r * u / 2       r ~ r`
    - `s = s * u / 2       s ~ s`

    Rounding errors accumulate faster with the second version, so it is only used for subsequent
    iterations within the same width integer. The first version is always used for the first
    iteration at a new width in order to avoid this accumulation.

    Goldschmidt has the advantage over Newton-Raphson that `sqrt(x)` and `1/sqrt(x)` are
    computed at the same time, i.e. there is no need to calculate `1/sqrt(x)` and invert it.
    */
    #[inline]
    pub const fn sqrt(x: f64) -> f64 {
        let mut ix = x.to_bits();

        // Top is the exponent and sign, which may or may not be shifted. If the float fits into a
        // `u32`, we can get by without paying shifting costs.
        let mut top = (ix >> SIG_BITS) as u32;
        let special_case = top.wrapping_sub(1) >= EXP_SAT - 1;

        // Handle NaN, zero, and out of domain (<= 0)
        if special_case {
            cold_path();

            // +/-0
            if ix << 1 == 0 {
                return x;
            }

            // Positive infinity
            if ix == EXP_MASK {
                return x;
            }

            // NaN or negative
            if ix > EXP_MASK {
                return f64::NAN;
            }

            // Normalize subnormals by multiplying by 1.0 << SIG_BITS (e.g. 0x1p52 for doubles).
            let scaled = x * from_parts(false, SIG_BITS + EXP_BIAS, 0);
            ix = scaled.to_bits();
            top = ex(scaled).wrapping_sub(SIG_BITS);
        }

        // Reduce arguments such that `x = 4^e * m`:
        //
        // - m_u2 ∈ [1, 4), a fixed point U2.BITS number
        // - 2^e is the exponent part of the result
        // We now know `x` is positive, so `top` is just its (biased) exponent
        let mut exp = top;
        // Construct a fixed point representation of the mantissa.
        let mut m_u2 = (ix | IMPLICIT_BIT) << EXP_BITS;
        let even = (exp & 1) != 0;
        if even {
            m_u2 >>= 1;
        }
        exp = exp.wrapping_add(EXP_SAT >> 1) >> 1;

        // Extract the top 6 bits of the significand with the lowest bit of the exponent.
        let i = ((ix >> (SIG_BITS - 6)) as usize) & 0b1111111;

        // Start with an initial guess for `r = 1 / sqrt(m)` from the table, and shift `m` as an
        // initial value for `s = sqrt(m)`. See the module documentation for details.
        let r2_u0 = (RSQRT_TAB[i] as u32) << (u32::BITS - 16);
        let s2_u2 = ((m_u2) >> (BITS - u32::BITS)) as u32;
        let (r2_u0, _s2_u2) = goldschmidt_r2(r2_u0, s2_u2, 2);

        // Perform final iterations at full width
        let r_u0: u64 = (r2_u0 as u64) << (BITS - u32::BITS);
        let s_u2: u64 = m_u2;
        let (_r_u0, s_u2) = goldschmidt_final(r_u0, s_u2, 2);

        // Shift back to mantissa position.
        let mut m = s_u2 >> (EXP_BITS - 2);

        // The musl source includes the following comment (with literals replaced):
        //
        // > s < sqrt(m) < s + 0x1.09p-SIG_BITS
        // > compute nearest rounded result: the nearest result to SIG_BITS bits is either s or
        // > s+0x1p-SIG_BITS, we can decide by comparing (2^SIG_BITS s + 0.5)^2 to 2^(2*SIG_BITS) m.
        //
        // Expanding this with , with `SIG_BITS = p` and adjusting based on the operations done to
        // `d0` and `d1`:
        //
        // - `2^(2p)m ≟ ((2^p)m + 0.5)^2`
        // - `2^(2p)m ≟ 2^(2p)m^2 + (2^p)m + 0.25`
        // - `2^(2p)m - m^2 ≟ (2^(2p) - 1)m^2 + (2^p)m + 0.25`
        // - `(1 - 2^(2p))m + m^2 ≟ (1 - 2^(2p))m^2 + (1 - 2^p)m + 0.25` (?)
        //
        // I do not follow how the rounding bit is extracted from this comparison with the below
        // operations. In any case, the algorithm is well tested.

        // The value needed to shift `m_u2` by to create `m*2^(2p)`. `2p = 2 * SIG_BITS`,
        // `BITS - 2` accounts for the offset that `m_u2` already has.
        let shift = 2 * SIG_BITS - BITS - 2;

        // `2^(2p)m - m^2`
        let d0 = (m_u2 << shift).wrapping_sub(m.wrapping_mul(m));
        // `m - 2^(2p)m + m^2`
        let d1 = m.wrapping_sub(d0);
        m += d1 >> (BITS - 1);
        m &= SIG_MASK;
        m |= (exp as u64) << SIG_BITS;

        let mut y = f64::from_bits(m);

        // Handle rounding and inexact. `(m + 1)^2 == 2^shift m` is exact; for all other cases, add
        // a tiny value to cause fenv effects.
        let d2 = d1.wrapping_add(m).wrapping_add(1);
        let mut tiny = if d2 == 0 {
            cold_path();
            0
        } else {
            IMPLICIT_BIT
        };

        tiny |= (d1 ^ d2) & SIGN_MASK;
        let t = f64::from_bits(tiny);
        y = y + t;

        y
    }

    const fn wmulh_u32(a: u32, b: u32) -> u32 {
        (((a as u64).wrapping_mul(b as u64)) >> 32) as u32
    }

    const fn wmulh_u64(a: u64, b: u64) -> u64 {
        (((a as u128).wrapping_mul(b as u128)) >> 64) as u64
    }

    #[inline]
    const fn goldschmidt_r2(mut r_u0: u32, mut s_u2: u32, count: u32) -> (u32, u32) {
        let three_u2 = (0b11u32) << (u32::BITS - 2);
        let mut u_u0 = r_u0;

        let mut i = 0;
        while i < count {
            // First iteration: `s = m*r` (`u_u0 = r_u0` set above)
            // Subsequent iterations: `s=s*u/2`
            s_u2 = wmulh_u32(s_u2, u_u0);

            // Perform `s /= 2` if:
            //
            // 1. This is not the first iteration (the first iteration is `s = m*r`)...
            // 2. ... and this is not the last set of iterations
            // 3. ... or, if this is the last set, it is not the last iteration
            //
            // This step is not performed for the final iteration because the shift is combined with
            // a later shift (moving `s` into the mantissa).
            if i > 0 {
                s_u2 <<= 1;
            }

            // u = 3 - s*r
            let d_u2 = wmulh_u32(s_u2, r_u0);
            u_u0 = three_u2.wrapping_sub(d_u2);

            // r = r*u/2
            r_u0 = wmulh_u32(r_u0, u_u0) << 1;

            i += 1;
        }

        (r_u0, s_u2)
    }

    #[inline]
    const fn goldschmidt_final(mut r_u0: u64, mut s_u2: u64, count: u32) -> (u64, u64) {
        let three_u2 = (0b11u64) << (u64::BITS - 2);
        let mut u_u0 = r_u0;

        let mut i = 0;
        while i < count {
            // First iteration: `s = m*r` (`u_u0 = r_u0` set above)
            // Subsequent iterations: `s=s*u/2`
            s_u2 = wmulh_u64(s_u2, u_u0);

            // Perform `s /= 2` if:
            //
            // 1. This is not the first iteration (the first iteration is `s = m*r`)...
            // 2. ... and this is not the last set of iterations
            // 3. ... or, if this is the last set, it is not the last iteration
            //
            // This step is not performed for the final iteration because the shift is combined with
            // a later shift (moving `s` into the mantissa).
            if i > 0 && i + 1 < count {
                s_u2 <<= 1;
            }

            // u = 3 - s*r
            let d_u2 = wmulh_u64(s_u2, r_u0);
            u_u0 = three_u2.wrapping_sub(d_u2);

            // r = r*u/2
            r_u0 = wmulh_u64(r_u0, u_u0) << 1;

            i += 1;
        }

        (r_u0, s_u2)
    }

    /// A U0.16 representation of `1/sqrt(x)`.
    ///
    /// The index is a 7-bit number consisting of a single exponent bit and 6 bits of significand.
    #[rustfmt::skip]
    static RSQRT_TAB: [u16; 128] = [
        0xb451, 0xb2f0, 0xb196, 0xb044, 0xaef9, 0xadb6, 0xac79, 0xab43,
        0xaa14, 0xa8eb, 0xa7c8, 0xa6aa, 0xa592, 0xa480, 0xa373, 0xa26b,
        0xa168, 0xa06a, 0x9f70, 0x9e7b, 0x9d8a, 0x9c9d, 0x9bb5, 0x9ad1,
        0x99f0, 0x9913, 0x983a, 0x9765, 0x9693, 0x95c4, 0x94f8, 0x9430,
        0x936b, 0x92a9, 0x91ea, 0x912e, 0x9075, 0x8fbe, 0x8f0a, 0x8e59,
        0x8daa, 0x8cfe, 0x8c54, 0x8bac, 0x8b07, 0x8a64, 0x89c4, 0x8925,
        0x8889, 0x87ee, 0x8756, 0x86c0, 0x862b, 0x8599, 0x8508, 0x8479,
        0x83ec, 0x8361, 0x82d8, 0x8250, 0x81c9, 0x8145, 0x80c2, 0x8040,
        0xff02, 0xfd0e, 0xfb25, 0xf947, 0xf773, 0xf5aa, 0xf3ea, 0xf234,
        0xf087, 0xeee3, 0xed47, 0xebb3, 0xea27, 0xe8a3, 0xe727, 0xe5b2,
        0xe443, 0xe2dc, 0xe17a, 0xe020, 0xdecb, 0xdd7d, 0xdc34, 0xdaf1,
        0xd9b3, 0xd87b, 0xd748, 0xd61a, 0xd4f1, 0xd3cd, 0xd2ad, 0xd192,
        0xd07b, 0xcf69, 0xce5b, 0xcd51, 0xcc4a, 0xcb48, 0xca4a, 0xc94f,
        0xc858, 0xc764, 0xc674, 0xc587, 0xc49d, 0xc3b7, 0xc2d4, 0xc1f4,
        0xc116, 0xc03c, 0xbf65, 0xbe90, 0xbdbe, 0xbcef, 0xbc23, 0xbb59,
        0xba91, 0xb9cc, 0xb90a, 0xb84a, 0xb78c, 0xb6d0, 0xb617, 0xb560,
    ];

    #[cfg(test)]
    mod tests {
        use super::*;

        use core::f64::consts::PI;

        /// Test behavior specified in IEEE 754 `squareRoot`.
        #[test]
        fn spec_test() {
            // Values that should return a NaN and raise invalid
            let nan = [f64::NEG_INFINITY, -1.0, f64::NAN, f64::MIN];

            // Values that return unaltered
            let roundtrip = [0.0f64, -0.0, f64::INFINITY];

            for x in nan {
                let val = sqrt(x);
                assert!(val.is_nan());
            }

            for x in roundtrip {
                let val = sqrt(x);
                assert_eq!(val.to_bits(), x.to_bits());
            }
        }

        #[test]
        fn sanity_check() {
            assert_eq!(sqrt(100.0f64).to_bits(), 10.0f64.to_bits());
            assert_eq!(sqrt(4.0f64).to_bits(), 2.0f64.to_bits());
        }

        #[test]
        #[allow(clippy::approx_constant)]
        fn conformance_tests() {
            let cases = [
                (PI, 0x3ffc5bf891b4ef6a_u64),
                (10000.0, 0x4059000000000000_u64),
                (f64::from_bits(0x0000000f), 0x1e7efbdeb14f4eda_u64),
                (f64::INFINITY, f64::INFINITY.to_bits()),
            ];

            for (input, output) in cases {
                assert_eq!(
                    sqrt(input).to_bits(),
                    output,
                    "input: {input:?} ({:#018x})",
                    input.to_bits()
                );
            }
        }
    }
}

pub use self::sqrt::*;

mod pow {
    /* origin: FreeBSD /usr/src/lib/msun/src/e_pow.c */
    /*
     * ====================================================
     * Copyright (C) 2004 by Sun Microsystems, Inc. All rights reserved.
     *
     * Permission to use, copy, modify, and distribute this
     * software is freely granted, provided that this notice
     * is preserved.
     * ====================================================
     */

    use super::*;

    const BP: [f64; 2] = [1.0, 1.5];
    const DP_H: [f64; 2] = [0.0, 5.84962487220764160156e-01]; /* 0x3fe2b803_40000000 */
    const DP_L: [f64; 2] = [0.0, 1.35003920212974897128e-08]; /* 0x3E4CFDEB, 0x43CFD006 */
    const TWO53: f64 = 9007199254740992.0; /* 0x43400000_00000000 */
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;

    // poly coefs for (3/2)*(log(x)-2s-2/3*s**3:
    const L1: f64 = 5.99999999999994648725e-01; /* 0x3fe33333_33333303 */
    const L2: f64 = 4.28571428578550184252e-01; /* 0x3fdb6db6_db6fabff */
    const L3: f64 = 3.33333329818377432918e-01; /* 0x3fd55555_518f264d */
    const L4: f64 = 2.72728123808534006489e-01; /* 0x3fd17460_a91d4101 */
    const L5: f64 = 2.30660745775561754067e-01; /* 0x3fcd864a_93c9db65 */
    const L6: f64 = 2.06975017800338417784e-01; /* 0x3fca7e28_4a454eef */
    const P1: f64 = 1.66666666666666019037e-01; /* 0x3fc55555_5555553e */
    const P2: f64 = -2.77777777770155933842e-03; /* 0xbf66c16c_16bebd93 */
    const P3: f64 = 6.61375632143793436117e-05; /* 0x3f11566a_af25de2c */
    const P4: f64 = -1.65339022054652515390e-06; /* 0xbebbbd41_c5d26bf1 */
    const P5: f64 = 4.13813679705723846039e-08; /* 0x3e663769_72bea4d0 */
    const LG2: f64 = 6.93147180559945286227e-01; /* 0x3fe62e42_fefa39ef */
    const LG2_H: f64 = 6.93147182464599609375e-01; /* 0x3fe62e43_00000000 */
    const LG2_L: f64 = -1.90465429995776804525e-09; /* 0xbe205c61_0ca86c39 */
    const OVT: f64 = 8.0085662595372944372e-017; /* -(1024-log2(ovfl+.5ulp)) */
    const CP: f64 = 9.61796693925975554329e-01; /* 0x3feec709_dc3a03fd =2/(3ln2) */
    const CP_H: f64 = 9.61796700954437255859e-01; /* 0x3feec709_e0000000 =(float)cp */
    const CP_L: f64 = -7.02846165095275826516e-09; /* 0xbe3e2fe0_145b01f5 =tail of cp_h*/
    const IVLN2: f64 = 1.44269504088896338700e+00; /* 0x3ff71547_652b82fe =1/ln2 */
    const IVLN2_H: f64 = 1.44269502162933349609e+00; /* 0x3ff71547_60000000 =24b 1/ln2*/
    const IVLN2_L: f64 = 1.92596299112661746887e-08; /* 0x3e54ae0b_f85ddf44 =1/ln2 tail*/

    /**
    pow(x,y) return x**y
                       n
    Method:  Let x =  2   * (1+f)
         1. Compute and return log2(x) in two pieces:
                 log2(x) = w1 + w2,
            where w1 has 53-24 = 29 bit trailing zeros.
         2. Perform y*log2(x) = n+y' by simulating multi-precision
            arithmetic, where |y'|<=0.5.
         3. Return x**y = 2**n*exp(y'*log2)

    Special cases:
         1.  (anything) ** 0  is 1
         2.  1 ** (anything)  is 1
         3.  (anything except 1) ** NAN is NAN
         4.  NAN ** (anything except 0) is NAN
         5.  +-(|x| > 1) **  +INF is +INF
         6.  +-(|x| > 1) **  -INF is +0
         7.  +-(|x| < 1) **  +INF is +0
         8.  +-(|x| < 1) **  -INF is +INF
         9.  -1          ** +-INF is 1
         10. +0 ** (+anything except 0, NAN)               is +0
         11. -0 ** (+anything except 0, NAN, odd integer)  is +0
         12. +0 ** (-anything except 0, NAN)               is +INF, raise divbyzero
         13. -0 ** (-anything except 0, NAN, odd integer)  is +INF, raise divbyzero
         14. -0 ** (+odd integer) is -0
         15. -0 ** (-odd integer) is -INF, raise divbyzero
         16. +INF ** (+anything except 0,NAN) is +INF
         17. +INF ** (-anything except 0,NAN) is +0
         18. -INF ** (+odd integer) is -INF
         19. -INF ** (anything) = -0 ** (-anything), (anything except odd integer)
         20. (anything) ** 1 is (anything)
         21. (anything) ** -1 is 1/(anything)
         22. (-anything) ** (integer) is (-1)**(integer)*(+anything**integer)
         23. (-anything except 0 and inf) ** (non-integer) is NAN

    Accuracy:
         pow(x,y) returns x**y nearly rounded. In particular
                         pow(integer,integer)
         always returns the correct integer provided it is
         representable.

    Constants :
    The hexadecimal values are the intended ones for the following
    constants. The decimal values may be used, provided that the
    compiler will convert from decimal to binary accurately enough
    to produce the hexadecimal values shown.
    */
    pub const fn pow(x: f64, y: f64) -> f64 {
        let t1: f64;
        let t2: f64;

        let (hx, lx) = ((x.to_bits() >> 32) as i32, x.to_bits() as u32);
        let (hy, ly) = ((y.to_bits() >> 32) as i32, y.to_bits() as u32);

        let mut ix = hx & 0x7fffffffi32;
        let iy = hy & 0x7fffffffi32;

        /* x**0 = 1, even if x is NaN */
        if ((iy as u32) | ly) == 0 {
            return 1.0;
        }

        /* 1**y = 1, even if y is NaN */
        if hx == 0x3ff00000 && lx == 0 {
            return 1.0;
        }

        /* NaN if either arg is NaN */
        if ix > 0x7ff00000
            || (ix == 0x7ff00000 && lx != 0)
            || iy > 0x7ff00000
            || (iy == 0x7ff00000 && ly != 0)
        {
            return x + y;
        }

        /* determine if y is an odd int when x < 0
         * yisint = 0       ... y is not an integer
         * yisint = 1       ... y is an odd int
         * yisint = 2       ... y is an even int
         */
        let mut yisint: i32 = 0;
        let mut k: i32;
        let mut j: i32;
        if hx < 0 {
            if iy >= 0x43400000 {
                yisint = 2; /* even integer y */
            } else if iy >= 0x3ff00000 {
                k = (iy >> 20) - 0x3ff; /* exponent */

                if k > 20 {
                    j = (ly >> (52 - k)) as i32;

                    if (j << (52 - k)) == (ly as i32) {
                        yisint = 2 - (j & 1);
                    }
                } else if ly == 0 {
                    j = iy >> (20 - k);

                    if (j << (20 - k)) == iy {
                        yisint = 2 - (j & 1);
                    }
                }
            }
        }

        if ly == 0 {
            /* special value of y */
            if iy == 0x7ff00000 {
                /* y is +-inf */

                return if ((ix - 0x3ff00000) | (lx as i32)) == 0 {
                    /* (-1)**+-inf is 1 */
                    1.0
                } else if ix >= 0x3ff00000 {
                    /* (|x|>1)**+-inf = inf,0 */
                    if hy >= 0 {
                        y
                    } else {
                        0.0
                    }
                } else {
                    /* (|x|<1)**+-inf = 0,inf */
                    if hy >= 0 {
                        0.0
                    } else {
                        -y
                    }
                };
            }

            if iy == 0x3ff00000 {
                /* y is +-1 */
                return if hy >= 0 { x } else { 1.0 / x };
            }

            if hy == 0x40000000 {
                /* y is 2 */
                return x * x;
            }

            if hy == 0x3fe00000 {
                /* y is 0.5 */
                if hx >= 0 {
                    /* x >= +0 */
                    return sqrt(x);
                }
            }
        }

        let mut ax = x.abs();
        if lx == 0 {
            /* special value of x */
            if ix == 0x7ff00000 || ix == 0 || ix == 0x3ff00000 {
                /* x is +-0,+-inf,+-1 */
                let mut z = ax;

                if hy < 0 {
                    /* z = (1/|x|) */
                    z = 1.0 / z;
                }

                if hx < 0 {
                    if ((ix - 0x3ff00000) | yisint) == 0 {
                        z = (z - z) / (z - z); /* (-1)**non-int is NaN */
                    } else if yisint == 1 {
                        z = -z; /* (x<0)**odd = -(|x|**odd) */
                    }
                }

                return z;
            }
        }

        let mut s = 1.0; /* sign of result */
        if hx < 0 {
            if yisint == 0 {
                /* (x<0)**(non-int) is NaN */
                return (x - x) / (x - x);
            }

            if yisint == 1 {
                /* (x<0)**(odd int) */
                s = -1.0;
            }
        }

        /* |y| is HUGE */
        if iy > 0x41e00000 {
            /* if |y| > 2**31 */
            if iy > 0x43f00000 {
                /* if |y| > 2**64, must o/uflow */
                if ix <= 0x3fefffff {
                    return if hy < 0 { HUGE * HUGE } else { TINY * TINY };
                }

                if ix >= 0x3ff00000 {
                    return if hy > 0 { HUGE * HUGE } else { TINY * TINY };
                }
            }

            /* over/underflow if x is not close to one */
            if ix < 0x3fefffff {
                return if hy < 0 {
                    s * HUGE * HUGE
                } else {
                    s * TINY * TINY
                };
            }
            if ix > 0x3ff00000 {
                return if hy > 0 {
                    s * HUGE * HUGE
                } else {
                    s * TINY * TINY
                };
            }

            /* now |1-x| is TINY <= 2**-20, suffice to compute
            log(x) by x-x^2/2+x^3/3-x^4/4 */
            let t = ax - 1.0; /* t has 20 trailing zeros */
            let w = (t * t) * (0.5 - t * (0.3333333333333333333333 - t * 0.25));
            let u = IVLN2_H * t; /* ivln2_h has 21 sig. bits */
            let v = t * IVLN2_L - w * IVLN2;
            t1 = with_set_low_word(u + v, 0);
            t2 = v - (t1 - u);
        } else {
            // double ss,s2,s_h,s_l,t_h,t_l;
            let mut n: i32 = 0;

            if ix < 0x00100000 {
                /* take care subnormal number */
                ax *= TWO53;
                n -= 53;
                ix = get_high_word(ax) as i32;
            }

            n += (ix >> 20) - 0x3ff;
            j = ix & 0x000fffff;

            /* determine interval */
            let k: i32;
            ix = j | 0x3ff00000; /* normalize ix */
            if j <= 0x3988E {
                /* |x|<sqrt(3/2) */
                k = 0;
            } else if j < 0xBB67A {
                /* |x|<sqrt(3)   */
                k = 1;
            } else {
                k = 0;
                n += 1;
                ix -= 0x00100000;
            }
            ax = with_set_high_word(ax, ix as u32);

            /* compute ss = s_h+s_l = (x-1)/(x+1) or (x-1.5)/(x+1.5) */
            let u = ax - BP[k as usize]; /* bp[0]=1.0, bp[1]=1.5 */
            let v = 1.0 / (ax + BP[k as usize]);
            let ss = u * v;
            let s_h = with_set_low_word(ss, 0);

            /* t_h=ax+bp[k] High */
            let t_h = with_set_high_word(
                0.0,
                ((ix as u32 >> 1) | 0x20000000) + 0x00080000 + ((k as u32) << 18),
            );
            let t_l = ax - (t_h - BP[k as usize]);
            let s_l = v * ((u - s_h * t_h) - s_h * t_l);

            /* compute log(ax) */
            let s2 = ss * ss;
            let mut r = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
            r += s_l * (s_h + ss);
            let s2 = s_h * s_h;
            let t_h = with_set_low_word(3.0 + s2 + r, 0);
            let t_l = r - ((t_h - 3.0) - s2);

            /* u+v = ss*(1+...) */
            let u = s_h * t_h;
            let v = s_l * t_h + t_l * ss;

            /* 2/(3log2)*(ss+...) */
            let p_h = with_set_low_word(u + v, 0);
            let p_l = v - (p_h - u);
            let z_h = CP_H * p_h; /* cp_h+cp_l = 2/(3*log2) */
            let z_l = CP_L * p_h + p_l * CP + DP_L[k as usize];

            /* log2(ax) = (ss+..)*2/(3*log2) = n + dp_h + z_h + z_l */
            let t = n as f64;
            t1 = with_set_low_word(((z_h + z_l) + DP_H[k as usize]) + t, 0);
            t2 = z_l - (((t1 - t) - DP_H[k as usize]) - z_h);
        }

        /* split up y into y1+y2 and compute (y1+y2)*(t1+t2) */
        let y1 = with_set_low_word(y, 0);
        let p_l = (y - y1) * t1 + y * t2;
        let mut p_h = y1 * t1;
        let z = p_l + p_h;
        let mut j = (z.to_bits() >> 32) as i32;
        let i = z.to_bits() as i32;

        if j >= 0x40900000 {
            /* z >= 1024 */
            if (j - 0x40900000) | i != 0 {
                /* if z > 1024 */
                return s * HUGE * HUGE; /* overflow */
            }

            if p_l + OVT > z - p_h {
                return s * HUGE * HUGE; /* overflow */
            }
        } else if (j & 0x7fffffff) >= 0x4090cc00 {
            /* z <= -1075 */

            if (((j as u32) - 0xc090cc00) | (i as u32)) != 0 {
                /* z < -1075 */
                return s * TINY * TINY; /* underflow */
            }

            if p_l <= z - p_h {
                return s * TINY * TINY; /* underflow */
            }
        }

        /* compute 2**(p_h+p_l) */
        let i = j & 0x7fffffffi32;
        k = (i >> 20) - 0x3ff;
        let mut n = 0;

        if i > 0x3fe00000 {
            /* if |z| > 0.5, set n = [z+0.5] */
            n = j + (0x00100000 >> (k + 1));
            k = ((n & 0x7fffffff) >> 20) - 0x3ff; /* new k for n */
            let t = with_set_high_word(0.0, (n & !(0x000fffff >> k)) as u32);
            n = ((n & 0x000fffff) | 0x00100000) >> (20 - k);
            if j < 0 {
                n = -n;
            }
            p_h -= t;
        }

        let t = with_set_low_word(p_l + p_h, 0);
        let u = t * LG2_H;
        let v = (p_l - (t - p_h)) * LG2 + t * LG2_L;
        let mut z = u + v;
        let w = v - (z - u);
        let t = z * z;
        let t1 = z - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
        let r = (z * t1) / (t1 - 2.0) - (w + z * w);
        z = 1.0 - (r - z);
        j = get_high_word(z) as i32;
        j += n << 20;

        if (j >> 20) <= 0 {
            /* subnormal output */
            z = scalbn(z, n);
        } else {
            z = with_set_high_word(z, j as u32);
        }

        s * z
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use core::f64::consts::{E, PI};

        const POS_ZERO: &[f64] = &[0.0];
        const NEG_ZERO: &[f64] = &[-0.0];
        const POS_ONE: &[f64] = &[1.0];
        const NEG_ONE: &[f64] = &[-1.0];
        const POS_FLOATS: &[f64] = &[99.0 / 70.0, E, PI];
        const NEG_FLOATS: &[f64] = &[-99.0 / 70.0, -E, -PI];
        const POS_SMALL_FLOATS: &[f64] = &[(1.0 / 2.0), f64::MIN_POSITIVE, f64::EPSILON];
        const NEG_SMALL_FLOATS: &[f64] = &[-(1.0 / 2.0), -f64::MIN_POSITIVE, -f64::EPSILON];
        const POS_EVENS: &[f64] = &[2.0, 6.0, 8.0, 10.0, 22.0, 100.0, f64::MAX];
        const NEG_EVENS: &[f64] = &[f64::MIN, -100.0, -22.0, -10.0, -8.0, -6.0, -2.0];
        const POS_ODDS: &[f64] = &[3.0, 7.0];
        const NEG_ODDS: &[f64] = &[-7.0, -3.0];
        const NANS: &[f64] = &[f64::NAN];
        const POS_INF: &[f64] = &[f64::INFINITY];
        const NEG_INF: &[f64] = &[f64::NEG_INFINITY];

        const ALL: &[&[f64]] = &[
            POS_ZERO,
            NEG_ZERO,
            NANS,
            NEG_SMALL_FLOATS,
            POS_SMALL_FLOATS,
            NEG_FLOATS,
            POS_FLOATS,
            NEG_EVENS,
            POS_EVENS,
            NEG_ODDS,
            POS_ODDS,
            NEG_INF,
            POS_INF,
            NEG_ONE,
            POS_ONE,
        ];
        const POS: &[&[f64]] = &[POS_ZERO, POS_ODDS, POS_ONE, POS_FLOATS, POS_EVENS, POS_INF];
        const NEG: &[&[f64]] = &[NEG_ZERO, NEG_ODDS, NEG_ONE, NEG_FLOATS, NEG_EVENS, NEG_INF];

        fn pow_test(base: f64, exponent: f64, expected: f64) {
            let res = pow(base, exponent);
            assert!(
                if expected.is_nan() {
                    res.is_nan()
                } else {
                    pow(base, exponent) == expected
                },
                "{base} ** {exponent} was {res} instead of {expected}",
            );
        }

        fn test_sets_as_base(sets: &[&[f64]], exponent: f64, expected: f64) {
            sets.iter()
                .for_each(|s| s.iter().for_each(|val| pow_test(*val, exponent, expected)));
        }

        fn test_sets_as_exponent(base: f64, sets: &[&[f64]], expected: f64) {
            sets.iter()
                .for_each(|s| s.iter().for_each(|val| pow_test(base, *val, expected)));
        }

        fn test_sets(
            sets: &[&[f64]],
            computed: &dyn Fn(f64) -> f64,
            expected: &dyn Fn(f64) -> f64,
        ) {
            sets.iter().for_each(|s| {
                s.iter().for_each(|val| {
                    let exp = expected(*val);
                    let res = computed(*val);

                    assert!(
                        if exp.is_nan() {
                            res.is_nan()
                        } else {
                            exp == res
                        },
                        "test for {val} was {res} instead of {exp}",
                    );
                })
            });
        }

        #[test]
        fn zero_as_exponent() {
            test_sets_as_base(ALL, 0.0, 1.0);
            test_sets_as_base(ALL, -0.0, 1.0);
        }

        #[test]
        fn one_as_base() {
            test_sets_as_exponent(1.0, ALL, 1.0);
        }

        #[test]
        fn nan_inputs() {
            // NAN as the base:
            // (f64::NAN ^ anything *but 0* should be f64::NAN)
            test_sets_as_exponent(f64::NAN, &ALL[2..], f64::NAN);

            // f64::NAN as the exponent:
            // (anything *but 1* ^ f64::NAN should be f64::NAN)
            test_sets_as_base(&ALL[..(ALL.len() - 2)], f64::NAN, f64::NAN);
        }

        #[test]
        fn infinity_as_base() {
            // Positive Infinity as the base:
            // (+Infinity ^ positive anything but 0 and f64::NAN should be +Infinity)
            test_sets_as_exponent(f64::INFINITY, &POS[1..], f64::INFINITY);

            // (+Infinity ^ negative anything except 0 and f64::NAN should be 0.0)
            test_sets_as_exponent(f64::INFINITY, &NEG[1..], 0.0);

            // Negative Infinity as the base:
            // (-Infinity ^ positive odd ints should be -Infinity)
            test_sets_as_exponent(f64::NEG_INFINITY, &[POS_ODDS], f64::NEG_INFINITY);

            // (-Infinity ^ anything but odd ints should be == -0 ^ (-anything))
            // We can lump in pos/neg odd ints here because they don't seem to
            // cause panics (div by zero) in release mode (I think).
            test_sets(ALL, &|v: f64| pow(f64::NEG_INFINITY, v), &|v: f64| {
                pow(-0.0, -v)
            });
        }

        #[test]
        fn infinity_as_exponent() {
            // Positive/Negative base greater than 1:
            // (pos/neg > 1 ^ Infinity should be Infinity - note this excludes f64::NAN as the base)
            test_sets_as_base(&ALL[5..(ALL.len() - 2)], f64::INFINITY, f64::INFINITY);

            // (pos/neg > 1 ^ -Infinity should be 0.0)
            test_sets_as_base(&ALL[5..ALL.len() - 2], f64::NEG_INFINITY, 0.0);

            // Positive/Negative base less than 1:
            let base_below_one = &[POS_ZERO, NEG_ZERO, NEG_SMALL_FLOATS, POS_SMALL_FLOATS];

            // (pos/neg < 1 ^ Infinity should be 0.0 - this also excludes f64::NAN as the base)
            test_sets_as_base(base_below_one, f64::INFINITY, 0.0);

            // (pos/neg < 1 ^ -Infinity should be Infinity)
            test_sets_as_base(base_below_one, f64::NEG_INFINITY, f64::INFINITY);

            // Positive/Negative 1 as the base:
            // (pos/neg 1 ^ Infinity should be 1)
            test_sets_as_base(&[NEG_ONE, POS_ONE], f64::INFINITY, 1.0);

            // (pos/neg 1 ^ -Infinity should be 1)
            test_sets_as_base(&[NEG_ONE, POS_ONE], f64::NEG_INFINITY, 1.0);
        }

        #[test]
        fn zero_as_base() {
            // Positive Zero as the base:
            // (+0 ^ anything positive but 0 and f64::NAN should be +0)
            test_sets_as_exponent(0.0, &POS[1..], 0.0);

            // (+0 ^ anything negative but 0 and f64::NAN should be Infinity)
            // (this should panic because we're dividing by zero)
            test_sets_as_exponent(0.0, &NEG[1..], f64::INFINITY);

            // Negative Zero as the base:
            // (-0 ^ anything positive but 0, f64::NAN, and odd ints should be +0)
            test_sets_as_exponent(-0.0, &POS[3..], 0.0);

            // (-0 ^ anything negative but 0, f64::NAN, and odd ints should be Infinity)
            // (should panic because of divide by zero)
            test_sets_as_exponent(-0.0, &NEG[3..], f64::INFINITY);

            // (-0 ^ positive odd ints should be -0)
            test_sets_as_exponent(-0.0, &[POS_ODDS], -0.0);

            // (-0 ^ negative odd ints should be -Infinity)
            // (should panic because of divide by zero)
            test_sets_as_exponent(-0.0, &[NEG_ODDS], f64::NEG_INFINITY);
        }

        #[test]
        fn special_cases() {
            // One as the exponent:
            // (anything ^ 1 should be anything - i.e. the base)
            test_sets(ALL, &|v: f64| pow(v, 1.0), &|v: f64| v);

            // Negative One as the exponent:
            // (anything ^ -1 should be 1/anything)
            test_sets(ALL, &|v: f64| pow(v, -1.0), &|v: f64| 1.0 / v);

            // Factoring -1 out:
            // (negative anything ^ integer should be (-1 ^ integer) * (positive anything ^ integer))
            [POS_ZERO, NEG_ZERO, POS_ONE, NEG_ONE, POS_EVENS, NEG_EVENS]
                .iter()
                .for_each(|int_set| {
                    int_set.iter().for_each(|int| {
                        test_sets(ALL, &|v: f64| pow(-v, *int), &|v: f64| {
                            pow(-1.0, *int) * pow(v, *int)
                        });
                    })
                });

            // Negative base (imaginary results):
            // (-anything except 0 and Infinity ^ non-integer should be NAN)
            NEG[1..(NEG.len() - 1)].iter().for_each(|set| {
                set.iter().for_each(|val| {
                    test_sets(&ALL[3..7], &|v: f64| pow(*val, v), &|_| f64::NAN);
                })
            });
        }

        #[test]
        fn normal_cases() {
            assert_eq!(pow(2.0, 20.0), (1 << 20) as f64);
            assert_eq!(pow(-1.0, 9.0), -1.0);
            assert!(pow(-1.0, 2.2).is_nan());
            assert!(pow(-1.0, -1.14).is_nan());
        }
    }
}

pub use self::pow::*;

mod log {
    /* origin: FreeBSD /usr/src/lib/msun/src/e_log2.c */
    /*
     * ====================================================
     * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
     *
     * Developed at SunSoft, a Sun Microsystems, Inc. business.
     * Permission to use, copy, modify, and distribute this
     * software is freely granted, provided that this notice
     * is preserved.
     * ====================================================
     */

    use core::f64;

    const IVLN2HI: f64 = 1.44269504072144627571e+00; /* 0x3ff71547, 0x65200000 */
    const IVLN2LO: f64 = 1.67517131648865118353e-10; /* 0x3de705fc, 0x2eefa200 */
    const LG1: f64 = 6.666666666666735130e-01; /* 3FE55555 55555593 */
    const LG2: f64 = 3.999999999940941908e-01; /* 3FD99999 9997FA04 */
    const LG3: f64 = 2.857142874366239149e-01; /* 3FD24924 94229359 */
    const LG4: f64 = 2.222219843214978396e-01; /* 3FCC71C5 1D8E78AF */
    const LG5: f64 = 1.818357216161805012e-01; /* 3FC74664 96CB03DE */
    const LG6: f64 = 1.531383769920937332e-01; /* 3FC39A09 D078C69F */
    const LG7: f64 = 1.479819860511658591e-01; /* 3FC2F112 DF3E5244 */

    /**
    Return the base 2 logarithm of x.

    Reduce `x` to `2^k (1+f)` and calculate `r = log(1+f) - f + f*f/2`
    as in log.c, then combine and scale in extra precision:
       `log2(x) = (f - f*f/2 + r)/log(2) + k`
    */
    pub const fn log2(mut x: f64) -> f64 {
        let x1p54 = f64::from_bits(0x4350000000000000); // 0x1p54 === 2 ^ 54

        let mut ui: u64 = x.to_bits();
        let hfsq: f64;
        let f: f64;
        let s: f64;
        let z: f64;
        let r: f64;
        let mut w: f64;
        let t1: f64;
        let t2: f64;
        let y: f64;
        let mut hi: f64;
        let lo: f64;
        let mut val_hi: f64;
        let mut val_lo: f64;
        let mut hx: u32;
        let mut k: i32;

        hx = (ui >> 32) as u32;
        k = 0;
        if hx < 0x00100000 || (hx >> 31) > 0 {
            if ui << 1 == 0 {
                return -1. / (x * x); /* log(+-0)=-inf */
            }
            if (hx >> 31) > 0 {
                return (x - x) / 0.0; /* log(-#) = NaN */
            }
            /* subnormal number, scale x up */
            k -= 54;
            x *= x1p54;
            ui = x.to_bits();
            hx = (ui >> 32) as u32;
        } else if hx >= 0x7ff00000 {
            return x;
        } else if hx == 0x3ff00000 && ui << 32 == 0 {
            return 0.;
        }

        /* reduce x into [sqrt(2)/2, sqrt(2)] */
        hx += 0x3ff00000 - 0x3fe6a09e;
        k += (hx >> 20) as i32 - 0x3ff;
        hx = (hx & 0x000fffff) + 0x3fe6a09e;
        ui = ((hx as u64) << 32) | (ui & 0xffffffff);
        x = f64::from_bits(ui);

        f = x - 1.0;
        hfsq = 0.5 * f * f;
        s = f / (2.0 + f);
        z = s * s;
        w = z * z;
        t1 = w * (LG2 + w * (LG4 + w * LG6));
        t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
        r = t2 + t1;

        /* hi+lo = f - hfsq + s*(hfsq+R) ~ log(1+f) */
        hi = f - hfsq;
        ui = hi.to_bits();
        ui &= (-1i64 as u64) << 32;
        hi = f64::from_bits(ui);
        lo = f - hi - hfsq + s * (hfsq + r);

        val_hi = hi * IVLN2HI;
        val_lo = (lo + hi) * IVLN2LO + lo * IVLN2HI;

        /* spadd(val_hi, val_lo, y), except for not using double_t: */
        y = k as f64;
        w = y + val_hi;
        val_lo += (y - w) + val_hi;
        val_hi = w;

        val_lo + val_hi
    }

    /**
    Compute the base `b` logarithm of `v`.
    */
    pub const fn log(v: f64, b: f64) -> f64 {
        log2(v) / log2(b)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn cases() {
            for (i, (case, base, expected)) in [
                (0.0, 0.0, f64::NAN),
                (0.0, 1.0, f64::NEG_INFINITY),
                (-0.0, 0.0, f64::NAN),
                (-0.0, 1.0, f64::NEG_INFINITY),
                (f64::INFINITY, 0.0, f64::NAN),
                (f64::INFINITY, 1.0, f64::INFINITY),
                (f64::NEG_INFINITY, 0.0, f64::NAN),
                (f64::NEG_INFINITY, 1.0, f64::NAN),
                (f64::NAN, 0.0, f64::NAN),
                (f64::NAN, 1.0, f64::NAN),
                (0.3740745044301024, 0.0, 0.0),
                (0.3740745044301024, 1.0, f64::NEG_INFINITY),
                (0.3740745044301024, 1.001, -983.7918599462805),
                (0.3740745044301024, 1.00001, -98330.52081878904),
                (0.3740745044301024, 1.000000001, -983300210.8340938),
                (0.3740745044301024, 2.0, -1.4186024545418017),
                (0.3740745044301024, 10.0, -0.42704189073963167),
                (0.7518324957739956, 0.0, 0.0),
                (0.7518324957739956, 1.0, f64::NEG_INFINITY),
                (0.7518324957739956, 1.001, -285.38432192921164),
                (0.7518324957739956, 1.00001, -28524.315102941237),
                (0.7518324957739956, 1.000000001, -285241701.3666506),
                (0.7518324957739956, 2.0, -0.41151682185969074),
                (0.7518324957739956, 10.0, -0.12387890710007803),
                (67.0512060439072, 0.0, -0.0),
                (67.0512060439072, 1.0, f64::INFINITY),
                (67.0512060439072, 1.001, 4207.558974817205),
                (67.0512060439072, 1.00001, 420547.7624018331),
                (67.0512060439072, 1.000000001, 4205456250.939661),
                (67.0512060439072, 2.0, 6.067191376874169),
                (67.0512060439072, 10.0, 1.8264065938729754),
                (0.0, 51.3852839123838, f64::NEG_INFINITY),
                (1.0, 51.3852839123838, 0.0),
                (1.001, 51.3852839123838, 0.0002537220276747872),
                (1.00001, 51.3852839123838, 0.0000025384759832150207),
                (1.000000001, 51.3852839123838, 0.0000000002538488884323403),
                (2.0, 51.3852839123838, 0.17595462683457902),
                (10.0, 51.3852839123838, 0.5845086183072099),
            ]
            .into_iter()
            .enumerate()
            {
                let actual = log(case, base);

                if expected.is_nan() && actual.is_nan() {
                    continue;
                }

                assert_eq!(
                    expected.to_bits(),
                    actual.to_bits(),
                    "{i}: {case}.log({base}) produced {actual} instead of {expected}"
                );
            }
        }
    }
}

pub use self::log::*;

#[inline(never)]
#[cold]
const fn cold_path() {}

#[inline]
const fn from_parts(negative: bool, exponent: u32, significand: u64) -> f64 {
    let sign = if negative { 1u64 } else { 0 };
    f64::from_bits(
        (sign << (BITS - 1))
            | (((exponent & EXP_SAT) as u64) << SIG_BITS)
            | (significand & SIG_MASK),
    )
}

#[inline]
const fn get_high_word(x: f64) -> u32 {
    (x.to_bits() >> 32) as u32
}

#[inline]
const fn with_set_high_word(f: f64, hi: u32) -> f64 {
    let mut tmp = f.to_bits();
    tmp &= 0x00000000_ffffffff;
    tmp |= (hi as u64) << 32;
    f64::from_bits(tmp)
}

#[inline]
const fn with_set_low_word(f: f64, lo: u32) -> f64 {
    let mut tmp = f.to_bits();
    tmp &= 0xffffffff_00000000;
    tmp |= lo as u64;
    f64::from_bits(tmp)
}

#[inline]
const fn ex(v: f64) -> u32 {
    ((v.to_bits() >> SIG_BITS) as u32) & EXP_SAT
}

const fn exp_unbiased(v: f64) -> i32 {
    ex(v) as i32 - (EXP_BIAS as i32)
}
