use speedy::Endianness;

/// Common conversion from bool to Endianness
/// true = LittleEndian, false = BigEndian
#[inline]
pub(crate) fn endianness_from_bool(little_endian: bool) -> Endianness {
    if little_endian {
        Endianness::LittleEndian
    } else {
        Endianness::BigEndian
    }
}

/// Trait for primitive types that can convert to/from byte arrays with endianness awareness.
pub trait EndianConvertible: Copy {
    type Bytes: Copy;

    fn to_le_bytes(self) -> Self::Bytes;
    fn to_be_bytes(self) -> Self::Bytes;
    fn from_le_bytes(bytes: Self::Bytes) -> Self;
    fn from_be_bytes(bytes: Self::Bytes) -> Self;
}

macro_rules! impl_endian_convertible {
    ($($ty:ty => $len:expr),+ $(,)?) => {
        $(impl EndianConvertible for $ty {
            type Bytes = [u8; $len];

            #[inline]
            fn to_le_bytes(self) -> Self::Bytes {
                self.to_le_bytes()
            }

            #[inline]
            fn to_be_bytes(self) -> Self::Bytes {
                self.to_be_bytes()
            }

            #[inline]
            fn from_le_bytes(bytes: Self::Bytes) -> Self {
                <$ty>::from_le_bytes(bytes)
            }

            #[inline]
            fn from_be_bytes(bytes: Self::Bytes) -> Self {
                <$ty>::from_be_bytes(bytes)
            }
        })+
    };
}

impl_endian_convertible! {
    u16 => 2,
    i16 => 2,
    u32 => 4,
    i32 => 4,
    u64 => 8,
    i64 => 8,
    f32 => 4,
    f64 => 8,
}

#[inline]
pub fn to_bytes<T: EndianConvertible>(value: T, endianness: Endianness) -> T::Bytes {
    match endianness {
        Endianness::LittleEndian => value.to_le_bytes(),
        Endianness::BigEndian => value.to_be_bytes(),
    }
}

#[inline]
pub fn from_bytes<T: EndianConvertible>(bytes: T::Bytes, endianness: Endianness) -> T {
    match endianness {
        Endianness::LittleEndian => T::from_le_bytes(bytes),
        Endianness::BigEndian => T::from_be_bytes(bytes),
    }
}

macro_rules! define_endian_wrappers {
    ($($ty:ty => $to_fn:ident, $from_fn:ident),+ $(,)?) => {
        $(
            #[inline]
            pub fn $to_fn(value: $ty, endianness: Endianness) -> <$ty as EndianConvertible>::Bytes {
                to_bytes(value, endianness)
            }

            #[inline]
            pub fn $from_fn(bytes: <$ty as EndianConvertible>::Bytes, endianness: Endianness) -> $ty {
                from_bytes(bytes, endianness)
            }
        )+
    };
}

define_endian_wrappers! {
    u16 => to_bytes_u16, from_bytes_u16,
    i16 => to_bytes_i16, from_bytes_i16,
    u32 => to_bytes_u32, from_bytes_u32,
    i32 => to_bytes_i32, from_bytes_i32,
    u64 => to_bytes_u64, from_bytes_u64,
    i64 => to_bytes_i64, from_bytes_i64,
    f32 => to_bytes_f32, from_bytes_f32,
    f64 => to_bytes_f64, from_bytes_f64,
}

/// IEEE 754 binary128 (the wire form of `float128`) built exactly from an `f64`.
#[inline]
pub fn to_bytes_f128(value: f64, endianness: Endianness) -> [u8; 16] {
    let bits = value.to_bits();
    let sign = (bits >> 63) as u128;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let mant = bits & ((1u64 << 52) - 1);
    let (exp128, mant128): (u128, u128) = match exp {
        0 if mant == 0 => (0, 0),
        0 => {
            let lz = mant.leading_zeros() - 12;
            let norm = (mant << (lz + 1)) & ((1u64 << 52) - 1);
            ((15360 - lz) as u128, (norm as u128) << 60)
        }
        0x7FF => (0x7FFF, (mant as u128) << 60),
        _ => ((exp - 1023 + 16383) as u128, (mant as u128) << 60),
    };
    let out = (sign << 127) | (exp128 << 112) | mant128;
    match endianness {
        Endianness::LittleEndian => out.to_le_bytes(),
        Endianness::BigEndian => out.to_be_bytes(),
    }
}

/// IEEE 754 binary128 narrowed to `f64` with round-to-nearest-even.
#[inline]
pub fn from_bytes_f128(bytes: [u8; 16], endianness: Endianness) -> f64 {
    let v = match endianness {
        Endianness::LittleEndian => u128::from_le_bytes(bytes),
        Endianness::BigEndian => u128::from_be_bytes(bytes),
    };
    let sign = ((v >> 127) as u64) << 63;
    let exp = ((v >> 112) & 0x7FFF) as i32;
    let mant = v & ((1u128 << 112) - 1);
    let bits = match exp {
        0x7FFF => {
            let m = (mant >> 60) as u64;
            sign | (0x7FFu64 << 52) | if mant != 0 && m == 0 { 1u64 << 51 } else { m }
        }
        0 => sign,
        _ => {
            let e = exp - 16383;
            if e > 1023 {
                sign | (0x7FFu64 << 52)
            } else if e >= -1022 {
                sign + (((e + 1023) as u64) << 52) + round_shift(mant, 60)
            } else {
                let shift = (-e - 962) as u32;
                let m = if shift >= 128 { 0 } else { round_shift((1u128 << 112) | mant, shift) };
                sign | m
            }
        }
    };
    f64::from_bits(bits)
}

fn round_shift(value: u128, shift: u32) -> u64 {
    let q = value >> shift;
    let rem = value & ((1u128 << shift) - 1);
    let half = 1u128 << (shift - 1);
    let up = rem > half || (rem == half && (q & 1) == 1);
    (q + up as u128) as u64
}
