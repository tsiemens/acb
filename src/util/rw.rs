use std::{fmt::Write, io, path::PathBuf};

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