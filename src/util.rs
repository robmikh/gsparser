use crate::bsp::BspReader;

pub fn resolve_map_entity_string<'a>(reader: &'a BspReader) -> std::borrow::Cow<'a, str> {
    let entities_bytes = reader.read_entities();
    resolve_null_terminated_string(entities_bytes)
}

pub fn resolve_null_terminated_string<'a>(bytes: &'a [u8]) -> std::borrow::Cow<'a, str> {
    resolve_null_terminated_string_with_warnings(bytes, false)
}

pub fn resolve_null_terminated_string_with_warnings<'a>(
    bytes: &'a [u8],
    print_warnings: bool,
) -> std::borrow::Cow<'a, str> {
    match null_terminated_bytes_to_str(bytes) {
        Ok(entities) => std::borrow::Cow::Borrowed(entities),
        Err(error) => {
            let start = error.str_error.valid_up_to();
            let end = start + error.str_error.error_len().unwrap_or(1);
            if print_warnings {
                println!("  WARNING: {:?}", error);
                println!("           error bytes: {:?}", &bytes[start..end]);
            }
            String::from_utf8_lossy(&bytes[..error.end])
        }
    }
}

#[derive(Debug)]
pub struct NullTerminatedStrError {
    pub end: usize,
    pub str_error: std::str::Utf8Error,
}

impl std::fmt::Display for NullTerminatedStrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for NullTerminatedStrError {}

pub fn null_terminated_bytes_to_str<'a>(
    bytes: &'a [u8],
) -> std::result::Result<&'a str, NullTerminatedStrError> {
    let end = bytes.iter().position(|x| *x == 0).unwrap_or(bytes.len());
    match std::str::from_utf8(&bytes[..end]) {
        Ok(string) => Ok(string),
        Err(err) => Err(NullTerminatedStrError {
            end,
            str_error: err,
        }),
    }
}
