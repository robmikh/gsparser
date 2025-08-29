use std::sync::atomic::{AtomicUsize, Ordering};

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

    pub fn read(&self, len: usize) -> std::io::Result<&[u8]> {
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

    pub fn read_u32_le(&self) -> std::io::Result<u32> {
        let bytes = self.read_and_copy::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }
}
