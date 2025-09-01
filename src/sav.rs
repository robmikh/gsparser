use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Write,
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

    pub fn read_to_end(&self) -> std::io::Result<&'a [u8]> {
        let start = self.position();
        let end = self.bytes.len();
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

    pub fn read_f32_le(&self) -> std::io::Result<f32> {
        let bytes = self.read_and_copy::<4>()?;
        Ok(f32::from_le_bytes(bytes))
    }

    pub fn read_null_terminated_str(&self) -> std::io::Result<&str> {
        let bytes = self.read_until_null()?;
        let result = str::from_utf8(bytes).into_io()?;
        Ok(result)
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
    pub fn parse(reader: &BytesReader<'a>) -> Result<Self, Box<dyn std::error::Error>> {
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

    pub fn get(&self, offset: u32) -> Option<&str> {
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
            Err(error) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{}", error),
            )),
        }
    }
}

trait SavFieldValue<'a>: Sized {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self>;
    fn record(&self, output: &mut String) -> std::fmt::Result;
}

impl<'a> SavFieldValue<'a> for &'a str {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        let bytes = reader.read_until_null()?;
        let result = str::from_utf8(bytes).into_io()?;
        Ok(result)
    }

    fn record(&self, output: &mut String) -> std::fmt::Result {
        write!(output, "\"{}\"", self)
    }
}

impl<'a> SavFieldValue<'a> for u32 {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        reader.read_u32_le()
    }

    fn record(&self, output: &mut String) -> std::fmt::Result {
        write!(output, "{} (0x{:X})", self, self)
    }
}

impl<'a> SavFieldValue<'a> for f32 {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        reader.read_f32_le()
    }

    fn record(&self, output: &mut String) -> std::fmt::Result {
        write!(output, "{}", self)
    }
}

impl<'a> SavFieldValue<'a> for &'a [u8] {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        let bytes = reader.read_to_end()?;
        Ok(bytes)
    }

    fn record(&self, output: &mut String) -> std::fmt::Result {
        write!(output, "{:?} ({:02X?})", self, self)
    }
}

impl<'a, const N: usize> SavFieldValue<'a> for [u8; N] {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        reader.read_and_copy()
    }

    fn record(&self, output: &mut String) -> std::fmt::Result {
        write!(output, "{:02X?}", self)
    }
}

impl<'a, const N: usize> SavFieldValue<'a> for [f32; N] {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        let mut result = [0.0f32; N];
        for i in 0..N {
            result[i] = reader.read_f32_le()?;
        }
        Ok(result)
    }

    fn record(&self, output: &mut String) -> std::fmt::Result {
        write!(output, "{:02X?}", self)
    }
}

fn resolve_string<'a>(bytes: &'a [u8]) -> Cow<'a, str> {
    match str::from_utf8(bytes) {
        Ok(entities) => Cow::Borrowed(entities),
        Err(_) => String::from_utf8_lossy(bytes),
    }
}

impl<'a> SavFieldValue<'a> for Cow<'a, str> {
    fn parse(reader: &BytesReader<'a>) -> std::io::Result<Self> {
        let bytes = reader.read_until_null()?;
        let result = resolve_string(bytes);
        Ok(result)
    }

    fn record(&self, output: &mut String) -> std::fmt::Result {
        write!(output, "\"{}\"", self)
    }
}

macro_rules! sav_tagged_struct{
    ($struct_name:ident ($struct_tag:literal) {
        $(
            $field_name:ident ($field_sav_name:literal) : $field_ty:ty
        ),*
    }) => {
        pub struct $struct_name<'a> {
            $(
                pub $field_name : Option<$field_ty>,
            )*
            _marker: std::marker::PhantomData<&'a str>,
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
                    _marker: std::marker::PhantomData
                })
            }

            pub fn record(&self, prefix: &str, output: &mut String) -> std::fmt::Result {
                writeln!(output, "{}{} ({}):", prefix, stringify!($struct_name), $struct_tag)?;
                $(
                    if let Some($field_name) = &self.$field_name {
                        write!(output, "{}  {}: ", prefix, stringify!($field_name))?;
                        $field_name.record(output)?;
                        writeln!(output)?;
                    }
                )*
                Ok(())
            }
        }
    };
}

macro_rules! sav_ordered_struct{
    ($struct_name:ident {
        $(
            $field_name:ident : $field_ty:ty
        ),*
    }) => {
        pub struct $struct_name<'a> {
            $(
                pub $field_name : $field_ty,
            )*
            _marker: std::marker::PhantomData<&'a str>,
        }

        impl<'a> $struct_name<'a> {
            pub fn parse(reader: &'a BytesReader<'a>) -> std::io::Result<Self> {
                // Read each field
                $(
                    let $field_name  = <$field_ty>::parse(reader)?;
                )*
                Ok(Self {
                    $(
                        $field_name,
                    )*
                    _marker: std::marker::PhantomData
                })
            }

            pub fn record(&self, prefix: &str, output: &mut String) -> std::fmt::Result {
                writeln!(output, "{}{}:", prefix, stringify!($struct_name))?;
                $(
                    write!(output, "{}  {}: ", prefix, stringify!($field_name))?;
                    self.$field_name.record(output)?;
                    writeln!(output)?;
                )*
                Ok(())
            }
        }
    };
}

sav_tagged_struct! {
    GameHeader ("GameHeader") {
        map_count ("mapCount"): u32,
        map_name ("mapName"): &'a str,
        comment ("comment"): &'a str
    }
}

sav_tagged_struct! {
    Globals ("GLOBAL") {
        len ("m_listCount"): u32
    }
}

sav_tagged_struct! {
    GlobalEntity ("GENT") {
        name ("name"): &'a str,
        level_name ("levelName"): &'a str,
        state ("state"): u32
    }
}

sav_tagged_struct! {
    Hl1SaveHeader ("Save Header") {
        skill_level ("skillLevel"): u32,
        entity_count ("entityCount"): u32,
        connection_count ("connectionCount"): u32,
        light_style_count ("lightStyleCount"): u32,
        time ("time"): u32,
        map_name ("mapName"): &'a str,
        sky_name ("skyName"): &'a str,
        sky_color_r ("skyColor_r"): &'a [u8],
        sky_color_g ("skyColor_g"): &'a [u8],
        sky_color_b ("skyColor_b"): &'a [u8],
        sky_vec_x ("skyVec_x"): f32,
        sky_vec_y ("skyVec_y"): f32,
        sky_vec_z ("skyVec_z"): f32
    }
}

sav_tagged_struct! {
    Adjacency ("ADJACENCY") {
        map_name ("mapName"): &'a str,
        landmark_name ("landmarkName"): &'a str,
        pent_landmark ("pentLandmark"): u32,
        vec_landmark_origin ("vecLandmarkOrigin"): [f32; 3]
    }
}

sav_tagged_struct! {
    EntityTable ("ETABLE") {
        location ("location"): u32,
        size ("size"): u32,
        class_name ("classname"): &'a str,
        flags ("flags"): u32,
        id ("id"): u32
    }
}

sav_tagged_struct! {
    LightStyle ("LIGHTSTYLE") {
        style ("style"): &'a str,
        index ("index"): u32
    }
}

sav_tagged_struct! {
    EntVars ("ENTVARS") {
        class_name ("classname"): &'a str,
        model_index ("modelindex"): u32,
        model ("model"): &'a str,
        abs_min ("absmin"): [f32; 3],
        abs_max ("absmax"): [f32; 3],
        mins ("mins"): [f32; 3],
        maxs ("maxs"): [f32; 3],
        size ("size"): [f32; 3],
        l_time ("ltime"): u32,
        next_think ("nextthink"): u32,
        solid ("solid"): u32,
        move_type ("move_type"): u32,
        flags ("flags"): u32
    }
}

pub struct UnknownTaggedStruct<'a, 'b> {
    pub tag: &'b str,
    pub fields: Vec<(&'b str, &'a [u8])>,
}

impl<'a, 'b> UnknownTaggedStruct<'a, 'b> {
    pub fn parse(
        reader: &'a BytesReader<'a>,
        string_table: &'b StringTable<'b>,
    ) -> std::io::Result<Self> {
        let always_4 = reader.read_u16_le()?;
        assert_eq!(always_4, 4);

        let token_offset = reader.read_u16_le()?;
        let token = string_table.get(token_offset as u32).unwrap();

        let fields_saved = reader.read_u16_le()?;
        // Not what this short is for
        let unknown = reader.read_u16_le()?;
        assert_eq!(unknown, 0);

        // Read each field
        let mut fields = Vec::with_capacity(fields_saved as usize);
        for _ in 0..fields_saved {
            let payload_size = reader.read_u16_le()?;
            let token_offset = reader.read_u16_le()?;
            let field_token = string_table.get(token_offset as u32).unwrap();
            let payload = reader.read(payload_size as usize)?;
            fields.push((field_token, payload));
        }

        Ok(Self { tag: token, fields })
    }

    pub fn record(&self, prefix: &str, output: &mut String) -> std::fmt::Result {
        writeln!(output, "{}{}:", prefix, self.tag)?;
        for (field_name, field_data) in &self.fields {
            writeln!(
                output,
                "{}  {}: {:?} ({:02X?})",
                prefix, field_name, field_data, field_data
            )?;
        }
        Ok(())
    }

    pub fn get(&self, field_name: &str) -> Option<&[u8]> {
        self.fields
            .iter()
            .find(|(name, _)| *name == field_name)
            .map(|(_, data)| *data)
    }

    pub fn get_str(&'a self, field_name: &str) -> std::io::Result<Option<Cow<'a, str>>> {
        if let Some(field_data) = self.get(field_name) {
            let reader = BytesReader::new(&field_data);
            Ok(Some(Cow::parse(&reader)?))
        } else {
            Ok(None)
        }
    }
}

pub struct HlBlock<'a> {
    pub name: &'a str,
    pub header_bytes: &'a [u8],
    pub block_offset: usize,
    pub block_bytes: &'a [u8],
}

impl<'a> HlBlock<'a> {
    pub fn parse(reader: &'a BytesReader<'a>) -> std::io::Result<Self> {
        let hl1_header_len = 260;
        let hl1_header = reader.read(hl1_header_len)?;
        let hl1_name_start = 0;
        let hl1_name_end = find_next_null(&hl1_header, hl1_name_start).unwrap_or(hl1_header.len());
        let hl1_name = str::from_utf8(&hl1_header[hl1_name_start..hl1_name_end]).into_io()?;

        let hl1_block_len = reader.read_u32_le()?;
        let block_offset = reader.position();
        let hl1_block = reader.read(hl1_block_len as usize)?;

        Ok(Self {
            name: hl1_name,
            header_bytes: hl1_header,
            block_offset,
            block_bytes: hl1_block,
        })
    }
}

sav_ordered_struct! {
    Hl1BlockHeader {
        magic: [u8; 4],
        version: u32,
        unknown_1: u32,
        expected_num_etables: u32
    }
}

impl<'a> Hl1BlockHeader<'a> {
    pub fn validate(&self) {
        assert_eq!(&self.magic, b"VALV");
        assert_eq!(self.version, 0x71);
    }
}
