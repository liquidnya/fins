use base64::{prelude::BASE64_STANDARD_NO_PAD, Engine as _};
use serde::{
    de::{Error, SeqAccess, Unexpected, Visitor},
    Deserializer,
};

struct ByteArrayVisitor<const N: usize>;

const fn hex_size(n: usize) -> usize {
    n * 2
}

const fn div_ceil(dividend: usize, diviser: usize) -> usize {
    dividend.div_ceil(diviser)
}

const fn base64_padded_size(n: usize) -> usize {
    div_ceil(n, 3) * 4
}

const fn base64_unpadded_size(n: usize) -> usize {
    div_ceil(n * 4, 3)
}

/*
const fn base64_padding_size(n: usize) -> usize {
    n * 2 % 3
}
*/

fn base64_check_padding<const N: usize>(v: &str) -> bool {
    v.as_bytes()[base64_unpadded_size(N)..base64_padded_size(N)]
        .iter()
        .all(|&c| c == b'=')
}

enum Decoding {
    Hex,
    Base64,
}

impl<'de, const N: usize> Visitor<'de> for ByteArrayVisitor<N> {
    type Value = [u8; N];

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match N {
            0 => write!(formatter, "bytes of length 0, an empty array, or an empty string"),
            2 | 4 => write!(formatter, "bytes or an array of length {} or a hex or base64 string of length {}", N, N * 2),
            _ => write!(formatter, "bytes or an array of length {} or a hex string of length {} or a base64 string of length {}", N, hex_size(N), base64_padded_size(N)),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut data = [0; N];
        let decoding: Option<Decoding> = match N {
            0 => None,
            2 | 4 => {
                if v.len() != N * 2 {
                    return Err(Error::invalid_length(v.len(), &self));
                } else if base64_check_padding::<N>(v) {
                    Some(Decoding::Base64)
                } else {
                    Some(Decoding::Hex)
                }
            }
            _ => {
                if v.len() == base64_padded_size(N) {
                    if base64_check_padding::<N>(v) {
                        Some(Decoding::Base64)
                    } else {
                        return Err(Error::invalid_value(
                            Unexpected::Other(&format!(
                                "invalid padding from index {} to {}",
                                base64_unpadded_size(N),
                                base64_padded_size(N) - 1
                            )),
                            &self,
                        ));
                    }
                } else if v.len() == hex_size(N) {
                    Some(Decoding::Hex)
                } else {
                    return Err(Error::invalid_length(v.len(), &self));
                }
            }
        };
        let len =
            match decoding {
                None => Ok(0),
                Some(Decoding::Base64) => BASE64_STANDARD_NO_PAD
                    .decode_slice(v, &mut data)
                    .map_err(|e| match e {
                        base64::DecodeSliceError::DecodeError(
                            base64::DecodeError::InvalidByte(index, data),
                        ) => Error::invalid_value(
                            Unexpected::Other(&format!(
                                "invalid byte {:#x} at index {}",
                                data, index
                            )),
                            &self,
                        ),
                        base64::DecodeSliceError::DecodeError(
                            base64::DecodeError::InvalidPadding,
                        ) => Error::invalid_value(Unexpected::Other("invalid padding"), &self),
                        base64::DecodeSliceError::DecodeError(
                            base64::DecodeError::InvalidLength(_),
                        ) => Error::invalid_length(v.len(), &self),
                        base64::DecodeSliceError::DecodeError(
                            base64::DecodeError::InvalidLastSymbol(index, data),
                        ) => Error::invalid_value(
                            Unexpected::Other(&format!(
                                "invalid last symbol {:#x} at index {}",
                                data, index
                            )),
                            &self,
                        ),
                        base64::DecodeSliceError::OutputSliceTooSmall => {
                            Error::invalid_length(v.len(), &self)
                        }
                    }),
                Some(Decoding::Hex) => {
                    hex::decode_to_slice(v, &mut data)
                        .map(|()| N)
                        .map_err(|e| match e {
                            hex::FromHexError::InvalidHexCharacter { c, index } => {
                                Error::invalid_value(
                                    Unexpected::Other(&format!(
                                        "invalid character `{}` at index {}",
                                        c, index
                                    )),
                                    &self,
                                )
                            }
                            hex::FromHexError::OddLength => Error::invalid_length(v.len(), &self),
                            hex::FromHexError::InvalidStringLength => {
                                Error::invalid_length(v.len(), &self)
                            }
                        })
                }
            }?;
        if len != N {
            return Err(Error::invalid_length(v.len(), &self));
        }
        Ok(data)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.try_into()
            .map_err(|_| Error::invalid_length(v.len(), &self))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        if let Some(len) = seq.size_hint() {
            if len != N {
                return Err(Error::invalid_length(len, &self));
            }
        }

        let mut data = [0; N];
        for (i, data) in data.iter_mut().enumerate() {
            if let Some(item) = seq.next_element::<u8>()? {
                *data = item;
            } else {
                return Err(Error::invalid_length(i, &self));
            }
        }

        let mut too_many_elements = 0;

        while seq.next_element::<u8>()?.is_some() {
            too_many_elements += 1;
        }

        if too_many_elements > 0 {
            return Err(Error::invalid_length(N + too_many_elements, &self));
        }

        Ok(data)
    }
}

pub fn deserialize<'de, T, D, const N: usize>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    [u8; N]: TryInto<T>,
    <[u8; N] as TryInto<T>>::Error: std::error::Error,
{
    let byte_array = deserializer.deserialize_any(ByteArrayVisitor::<N>)?;
    let result = byte_array.try_into();
    result.map_err(Error::custom)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
