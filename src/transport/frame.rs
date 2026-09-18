use std::io::{self, Read, Write};

pub const MAX_FRAME_LEN: u32 = 1 << 20;

#[derive(Debug)]
pub enum FrameError {
    Io(io::Error),
    Oversized(u32),
    Truncated,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "i/o error: {e}"),
            Self::Oversized(len) => write!(f, "frame of {len} bytes exceeds the limit"),
            Self::Truncated => write!(f, "stream ended in the middle of a frame"),
        }
    }
}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), FrameError> {
    let len = u32::try_from(payload.len()).map_err(|_| FrameError::Oversized(u32::MAX))?;
    if len > MAX_FRAME_LEN {
        return Err(FrameError::Oversized(len));
    }
    writer.write_all(&len.to_ne_bytes())?;
    writer.write_all(payload)?;
    Ok(())
}

pub fn read_frame<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, FrameError> {
    let mut header = [0u8; 4];
    let mut filled = 0;
    while filled < header.len() {
        match reader.read(&mut header[filled..]) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(FrameError::Truncated),
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(FrameError::Io(e)),
        }
    }

    let len = u32::from_ne_bytes(header);
    if len > MAX_FRAME_LEN {
        return Err(FrameError::Oversized(len));
    }

    let mut payload = vec![0u8; len as usize];
    let mut filled = 0;
    while filled < payload.len() {
        match reader.read(&mut payload[filled..]) {
            Ok(0) => return Err(FrameError::Truncated),
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(FrameError::Io(e)),
        }
    }
    Ok(Some(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Chunked<'a> {
        data: &'a [u8],
        max: usize,
    }

    impl Read for Chunked<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = buf.len().min(self.max).min(self.data.len());
            buf[..n].copy_from_slice(&self.data[..n]);
            self.data = &self.data[n..];
            Ok(n)
        }
    }

    fn encoded(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        write_frame(&mut out, payload).unwrap();
        out
    }

    #[test]
    fn writes_a_native_endian_length_prefix() {
        let mut out = Vec::new();
        write_frame(&mut out, b"hello").unwrap();
        assert_eq!(&out[..4], &5u32.to_ne_bytes());
        assert_eq!(&out[4..], b"hello");
    }

    #[test]
    fn reads_consecutive_frames() {
        let mut wire = encoded(b"one");
        wire.extend(encoded(b"two"));
        let mut reader = io::Cursor::new(wire);

        assert_eq!(read_frame(&mut reader).unwrap().unwrap(), b"one");
        assert_eq!(read_frame(&mut reader).unwrap().unwrap(), b"two");
        assert!(read_frame(&mut reader).unwrap().is_none());
    }

    #[test]
    fn reassembles_a_frame_split_across_reads() {
        let wire = encoded(b"split frame");
        let mut reader = Chunked {
            data: &wire,
            max: 1,
        };
        assert_eq!(read_frame(&mut reader).unwrap().unwrap(), b"split frame");
    }

    #[test]
    fn rejects_an_oversized_frame() {
        let mut wire = (MAX_FRAME_LEN + 1).to_ne_bytes().to_vec();
        wire.extend_from_slice(b"x");
        let mut reader = io::Cursor::new(wire);
        assert!(matches!(
            read_frame(&mut reader).unwrap_err(),
            FrameError::Oversized(_)
        ));
    }

    #[test]
    fn detects_a_truncated_frame() {
        let mut wire = encoded(b"hello");
        wire.truncate(wire.len() - 2);
        let mut reader = io::Cursor::new(wire);
        assert!(matches!(
            read_frame(&mut reader).unwrap_err(),
            FrameError::Truncated
        ));
    }

    #[test]
    fn reports_a_clean_end_of_stream() {
        let mut reader = io::Cursor::new(Vec::new());
        assert!(read_frame(&mut reader).unwrap().is_none());
    }
}
