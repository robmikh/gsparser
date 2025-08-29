use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct BytesReader<'a> {
    bytes: &'a [u8],
    current: AtomicUsize,
}

impl<'a> BytesReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            current: AtomicUsize::new(0),
        }
    }

    pub fn position(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }

    pub fn read_until_null(&self) -> std::io::Result<&'a [u8]> {
        let start = self.position();
        let mut current = start;
        let mut end = None;
        while current < self.bytes.len() {
            if self.bytes[current] == 0 {
                end = Some(current);
                break;
            }
            current += 1;
        }
        if let Some(end) = end {
            let bytes = self.read(end - start)?;
            Ok(bytes)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Never found null byte!",
            ))
        }
    }

    pub fn read(&self, len: usize) -> std::io::Result<&'a [u8]> {
        let start = self.position();
        let end = start + len;
        if end > self.bytes.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Could not read all {} byte(s)!", len),
            ));
        }
        self.current.store(end, Ordering::SeqCst);
        Ok(&self.bytes[start..end])
    }

    pub fn read_and_copy<const N: usize>(&self) -> std::io::Result<[u8; N]> {
        let bytes = self.read(N)?;
        let mut result = [0u8; N];
        result.copy_from_slice(bytes);
        Ok(result)
    }

    pub fn read_u16_le(&self) -> std::io::Result<u16> {
        let bytes = self.read_and_copy::<2>()?;
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn read_u32_le(&self) -> std::io::Result<u32> {
        let bytes = self.read_and_copy::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }
}

pub struct SavHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub global_entities_len: u32,
}

impl SavHeader {
    pub fn parse(reader: &BytesReader) -> std::io::Result<Self> {
        let magic = reader.read_and_copy()?;
        assert_eq!(&magic, b"JSAV");
        let version = reader.read_u32_le()?;
        assert_eq!(version, 0x71);
        let door_info_len = reader.read_u32_le()?;
        Ok(Self {
            magic,
            version,
            global_entities_len: door_info_len,
        })
    }
}

pub struct StringTable<'a> {
    table: HashMap<u32, &'a str>,
}

impl<'a> StringTable<'a> {
    pub fn parse(reader: &'a BytesReader<'a>) -> Result<Self, Box<dyn std::error::Error>> {
        let _token_count = reader.read_u32_le()?;
        let tokens_size = reader.read_u32_le()?;
        let string_table_bytes = reader.read(tokens_size as usize)?;
        let tokens = Self::parse_data(string_table_bytes)?;
        Ok(tokens)
    }

    fn parse_data(bytes: &'a [u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut table = HashMap::new();
        let mut current = 0;
        let mut num = 0;
        while current < bytes.len() {
            if bytes[current] != 0 {
                let start = current;
                let end = find_next_null(&bytes, start).unwrap();
                let string = str::from_utf8(&bytes[start..end])?;
                let previous = table.insert(num, string);
                assert!(previous.is_none());
                current = end;
            }
            current += 1;
            num += 1;
        }
        Ok(Self { table })
    }

    pub fn get(&'a self, offset: u32) -> Option<&'a str> {
        self.table.get(&offset).map(|x| *x)
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn get_sorted_keys(&self) -> Vec<u32> {
        let mut keys: Vec<u32> = self.table.keys().map(|x| *x).collect();
        keys.sort();
        keys
    }
}

pub fn find_next_null(bytes: &[u8], start: usize) -> Option<usize> {
    let mut end = start;
    while end < bytes.len() {
        if bytes[end] == 0 {
            return Some(end);
        }
        end += 1;
    }
    None
}

pub fn find_next_non_null(bytes: &[u8], start: usize) -> Option<usize> {
    let mut end = start;
    while end < bytes.len() {
        if bytes[end] != 0 {
            return Some(end);
        }
        end += 1;
    }
    None
}


trait IntoIo<T> {
    fn into_io(self) -> std::io::Result<T>;
}

impl<T> IntoIo<T> for Result<T, std::str::Utf8Error> {
    fn into_io(self) -> std::io::Result<T> {
        match self {
            Ok(result) => Ok(result),
            Err(error) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{}", error),
                ))
            }
        }
    }
}

trait SavFieldValue<'a>: Sized {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self>;
}

impl<'a> SavFieldValue<'a> for &'a str {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        let bytes = reader.read_until_null()?;
        let result = str::from_utf8(bytes).into_io()?;
        Ok(result)
    }
}

impl<'a> SavFieldValue<'a> for u32 {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        reader.read_u32_le()
    }
}

macro_rules! sav_struct{
    ($struct_name:ident ($struct_tag:literal) {
        $(
            $field_name:ident ($field_sav_name:literal) : $field_ty:ty
        ),*
    }) => {
        pub struct $struct_name<'a> {
            $(
                pub $field_name : Option<$field_ty>,
            )*
        }

        impl<'a> $struct_name<'a> {
            pub fn parse(reader: &'a BytesReader<'a>, string_table: &StringTable) -> std::io::Result<Self> {
                let always_4 = reader.read_u16_le()?;
                assert_eq!(always_4, 4);

                let token_offset = reader.read_u16_le()?;
                let token = string_table.get(token_offset as u32).unwrap();
                assert_eq!(token, $struct_tag);

                let fields_saved = reader.read_u16_le()?;
                // Not what this short is for
                let unknown = reader.read_u16_le()?;
                assert_eq!(unknown, 0);

                // Read each field
                $(
                    let mut $field_name: Option<$field_ty> = None;
                )*
                for _ in 0..fields_saved {
                    let payload_size = reader.read_u16_le()?;
                    let token_offset = reader.read_u16_le()?;
                    let field_token = string_table.get(token_offset as u32).unwrap();
                    let payload = reader.read(payload_size as usize)?;
                    let payload_reader = BytesReader::new(payload);

                    match field_token {
                        $(
                            $field_sav_name => {
                                $field_name = Some(<$field_ty>::parse(&payload_reader)?);
                            }
                        )*
                        _ => return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Property \"{}\" not recognized for struct \"{}\"!", field_token, token)))
                    }
                }

                Ok(Self {
                    $(
                        $field_name,
                    )*
                })
            }
        }
    };
}

sav_struct!{
    GameHeader ("GameHeader") {
        map_count ("mapCount"): u32,
        map_name ("mapName"): &'a str,
        comment ("comment"): &'a str
    }
}