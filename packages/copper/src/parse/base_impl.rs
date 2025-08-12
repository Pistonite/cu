use std::path::PathBuf;

use super::Parse;

macro_rules! blanket_parse_borrowed {
    () => {
        fn parse_borrowed(x: &str) -> crate::Result<Self::Output> {
            Self::parse_owned(x.to_owned())
        }
    };
}
macro_rules! blanket {
    ($($t:ty),* $(,)?) => { $(
        impl Parse for $t {
            type Output = Self;
            fn parse_borrowed(x: &str) -> crate::Result<Self> {
                use crate::Context as _;
                <Self as std::str::FromStr>::from_str(x).context(concat!("failed to parse ", stringify!($t)))
            }
        }
    )* };
}

// bool: empty, 0/1, false/true, case-insensitive
impl Parse for bool {
    type Output = Self;
    #[rustfmt::skip]
    fn parse_borrowed(x: &str) -> crate::Result<Self> {
        enum S {
            Empty, Zero, One,
            T1, T2, T3, True,
            F1, F2, F3, F4, False,
            Error,
        }
        let mut s = S::Empty;
        for c in x.trim().as_bytes() {
            match s {
                S::Error => break,
                S::Empty => match *c {
                    b'0' => s = S::Zero,
                    b'1' => s = S::One,
                    b't' | b'T' => s = S::T1,
                    b'f' | b'F' => s = S::F1,
                    _ => { s = S::Error; break; }
                }
                S::Zero | S::One | S::True | S::False => { s = S::Error; break; }
                S::T1 => match *c {
                    b'r' | b'R' => s = S::T2,
                    _ => { s = S::Error; break; }
                }
                S::T2 => match *c {
                    b'u' | b'U' => s = S::T3,
                    _ => { s = S::Error; break; }
                }
                S::T3 => match *c {
                    b'e' | b'E' => s = S::True,
                    _ => { s = S::Error; break; }
                }
                S::F1 => match *c {
                    b'a' | b'A' => s = S::F2,
                    _ => { s = S::Error; break; }
                }
                S::F2 => match *c {
                    b'l' | b'L' => s = S::F3,
                    _ => { s = S::Error; break; }
                }
                S::F3 => match *c {
                    b's' | b'S' => s = S::F4,
                    _ => { s = S::Error; break; }
                }
                S::F4 => match *c {
                    b'e' | b'E' => s = S::False,
                    _ => { s = S::Error; break; }
                }
            }
        }

        match s {
            S::Empty | S::Zero | S::False => Ok(false),
            S::One | S::True => Ok(true),
            _ => crate::bail!("invalid bool format"),
        }
    }
}

macro_rules! impl_parse_number {
    ($t:ty, $parse:ident) => {
        impl Parse for $t {
            type Output = Self;
            #[inline(always)]
            fn parse_borrowed(x: &str) -> crate::Result<Self> {
                $parse(x)
            }
        }
    };
}
impl_parse_number!(i8, parse_i8);
impl_parse_number!(i16, parse_i16);
impl_parse_number!(i32, parse_i32);
impl_parse_number!(i64, parse_i64);
impl_parse_number!(i128, parse_i128);
impl_parse_number!(isize, parse_isize);
impl_parse_number!(u8, parse_u8);
impl_parse_number!(u16, parse_u16);
impl_parse_number!(u32, parse_u32);
impl_parse_number!(u64, parse_u64);
impl_parse_number!(u128, parse_u128);
impl_parse_number!(usize, parse_usize);
blanket!(f32, f64);

macro_rules! parse_unsigned {
    ($t:ty, $parse:ident) => {
        fn $parse(x: &str) -> crate::Result<$t> {
            let x = x.trim();
            if x.starts_with('-') {
                crate::bail!("expecting unsigned integer");
            }
            // signed - only allow decimal
            if x.starts_with('+') {
                return Ok(x.parse::<$t>()?);
            }
            if let Some(x2) = x.strip_prefix('0') {
                if let Some(hex) = x2.strip_prefix(['x', 'X']) {
                    return Ok(<$t>::from_str_radix(hex, 16)?);
                }
                if let Some(oct) = x2.strip_prefix(['o', 'O']) {
                    return Ok(<$t>::from_str_radix(oct, 8)?);
                }
                if let Some(bin) = x2.strip_prefix(['b', 'B']) {
                    return Ok(<$t>::from_str_radix(bin, 2)?);
                }
            }
            Ok(x.parse::<$t>()?)
        }
    };
    ($t:ty, $parse:ident, $delegate:ident, $delegate_ty:ty) => {
        fn $parse(x: &str) -> crate::Result<$t> {
            let x = $delegate(x)?;
            if x > <$t>::MAX as $delegate_ty {
                crate::bail!("input out of range: max={}", <$t>::MAX);
            }
            Ok(x as $t)
        }
    };
}
parse_unsigned!(u8, parse_u8, parse_u32, u32);
parse_unsigned!(u16, parse_u16, parse_u32, u32);
parse_unsigned!(u32, parse_u32);
parse_unsigned!(u64, parse_u64);
parse_unsigned!(u128, parse_u128);
fn parse_usize(x: &str) -> crate::Result<usize> {
    #[cfg(target_pointer_width = "64")]
    {
        Ok(parse_u64(x)? as usize)
    }
    #[cfg(not(target_pointer_width = "64"))]
    {
        parse_unsigned!(usize, parse);
        parse(x)
    }
}
macro_rules! parse_signed {
    ($t:ty, $parse:ident) => {
        fn $parse(x: &str) -> crate::Result<$t> {
            let x = x.trim();
            // signed - only allow decimal
            if x.starts_with(['-', '+']) {
                return Ok(x.parse::<$t>()?);
            }
            if let Some(x2) = x.strip_prefix('0') {
                if let Some(hex) = x2.strip_prefix(['x', 'X']) {
                    return Ok(<$t>::from_str_radix(hex, 16)?);
                }
                if let Some(oct) = x2.strip_prefix(['o', 'O']) {
                    return Ok(<$t>::from_str_radix(oct, 8)?);
                }
                if let Some(bin) = x2.strip_prefix(['b', 'B']) {
                    return Ok(<$t>::from_str_radix(bin, 2)?);
                }
            }
            Ok(x.parse::<$t>()?)
        }
    };
    ($t:ty, $parse:ident, $delegate:ident, $delegate_ty:ty) => {
        fn $parse(x: &str) -> crate::Result<$t> {
            let x = $delegate(x)?;
            if x > <$t>::MAX as $delegate_ty || x < <$t>::MIN as $delegate_ty {
                crate::bail!("input out of range: min={}, max={}", <$t>::MIN, <$t>::MAX);
            }
            Ok(x as $t)
        }
    };
}
parse_signed!(i8, parse_i8, parse_i32, i32);
parse_signed!(i16, parse_i16, parse_i32, i32);
parse_signed!(i32, parse_i32);
parse_signed!(i64, parse_i64);
parse_signed!(i128, parse_i128);
fn parse_isize(x: &str) -> crate::Result<isize> {
    #[cfg(target_pointer_width = "64")]
    {
        Ok(parse_i64(x)? as isize)
    }
    #[cfg(not(target_pointer_width = "64"))]
    {
        parse_signed!(isize, parse);
        parse(x)
    }
}

impl Parse for String {
    type Output = Self;
    blanket_parse_borrowed!();
    fn parse_owned(x: String) -> crate::Result<Self> {
        Ok(x)
    }
}

impl Parse for PathBuf {
    type Output = Self;
    blanket_parse_borrowed!();
    fn parse_owned(x: String) -> crate::Result<Self> {
        Ok(x.into())
    }
}
