use std::{fmt::Write, fs::File, io, path::PathBuf};

use super::rc::{RcRefCell, RcRefCellT};

pub struct StringBuffer {
    s: String,
}

impl StringBuffer {
    pub fn new() -> StringBuffer {
        StringBuffer{s: String::new()}
    }

    pub fn as_str(&self) -> &str {
        self.s.as_str()
    }

    pub fn clear(&mut self) {
        self.s = String::new();
    }
}

// String only implements fmt::Write
impl io::Write for StringBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let str_rep = std::str::from_utf8(buf)
            .map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, e)
            })?;
        let res = self.s.write_str(str_rep);
        match res {
            Ok(_) => Ok(buf.len()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct StrReader<'a> {
    s: &'a str,
    cursor: usize,
}

impl <'a> From<&'a str> for StrReader<'a> {
    fn from(value: &'a str) -> Self {
        StrReader{ s: value, cursor: 0 }
    }
}

impl <'a> io::Read for StrReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes = self.s.as_bytes();
        let len = bytes.len();
        if self.cursor >= len {
            // No more data to read
            return Ok(0);
        }

        let remaining = len - self.cursor;
        let to_read = buf.len().min(remaining);
        buf[..to_read].copy_from_slice(&bytes[self.cursor..self.cursor + to_read]);
        self.cursor += to_read;
        Ok(to_read)
    }
}


// For convenience, so we can pass around a shared stream writer.
//
// One use is to capture errors of interest to users, so they can
// be presented either to the stderr, or buffer them to later
// show in the web UI.
pub struct WriteHandle {
    w: RcRefCell<dyn io::Write>,
}

impl WriteHandle {
    pub fn stdout_write_handle() -> WriteHandle {
        WriteHandle{
            w: RcRefCellT::new(io::stdout())
        }
    }

    pub fn stderr_write_handle() -> WriteHandle {
        WriteHandle{
            w: RcRefCellT::new(io::stderr())
        }
    }

    pub fn string_buff_write_handle() -> (WriteHandle, RcRefCell<StringBuffer>) {
        let buffer =
            RcRefCellT::new(StringBuffer::new());
        let h = WriteHandle{
            w: buffer.clone()
        };
        (h, buffer)
    }

    pub fn file_write_handle(f: File) -> WriteHandle {
        WriteHandle{
            w: RcRefCellT::new(f)
        }
    }

    pub fn empty_write_handle() -> WriteHandle {
        WriteHandle{
            w: RcRefCellT::new(io::empty())
        }
    }
}

impl io::Write for WriteHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Trace here, since tests should generally disable the error writer
        // or use a string buffer.
        // The test framework cannot capture direct writes to stdout or stderr,
        // only writes through print/println/eprintln.
        tracing::info!("WriteHandle::write {}", { let mut b = StringBuffer::new(); let _ = b.write(buf); b }.as_str() );
        self.w.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.borrow_mut().flush()
    }
}

// Generally, this will represent a file that has been opened,
// where we want to track the name along with it.
// Though it may be pre-read, in which case, we can just store
// the string.
pub enum DescribedReader {
    String((String, String)),
    FilePath(PathBuf),
}

impl DescribedReader {
    pub fn from_string(desc: String, data: String) -> DescribedReader {
        DescribedReader::String((desc, data))
    }

    pub fn from_file_path(path: PathBuf) -> DescribedReader {
        DescribedReader::FilePath(path)
    }

    pub fn desc(&self) -> &str {
        match self {
            DescribedReader::String((name, _)) => name,
            DescribedReader::FilePath(path) =>
                path.to_str().unwrap_or("<unknown path>"),
        }
    }

    pub fn reader<'a>(&'a self) -> Result<Box<dyn io::Read + 'a>,
                                          std::io::Error> {
        match self {
            DescribedReader::String((_, text)) => {
                Ok(Box::new(StrReader::from(text.as_str())))
            },
            DescribedReader::FilePath(path) => {
                match std::fs::File::open(path) {
                    Ok(x) => Ok(Box::new(x)),
                    Err(e) => Err(e),
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{StringBuffer, WriteHandle};

    #[test]
    fn test_string_buffer() {
        let mut buff = StringBuffer::new();
        let _ = write!(buff, "Some {}", "text");
        let _ = writeln!(buff, " 1");
        assert_eq!(buff.as_str(), "Some text 1\n");
    }

    #[test]
    fn test_write_handle() {
        let (mut handle, buff)
            = WriteHandle::string_buff_write_handle();
        let _ = write!(handle, "Some {}", "text");
        let _ = writeln!(handle, " 1");
        assert_eq!(buff.borrow().as_str(), "Some text 1");
    }
}