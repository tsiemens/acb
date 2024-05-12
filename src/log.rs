use std::{io, sync::Mutex};

use lazy_static::lazy_static;
use tracing::info;

use crate::util::{rc::{RcRefCell, RcRefCellT}, rw::StringBuffer};

// For convenience, so we can pass around a shared stream writer.
//
// Used to capture errors of interest to users, so they can
// be presented either to the stderr, or buffer them to later
// show in the web UI.
pub struct WriteHandle {
    w: RcRefCell<dyn io::Write>,
}

impl WriteHandle {
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
        info!("WriteHandle::write {}", { let mut b = StringBuffer::new(); let _ = b.write(buf); b }.as_str() );
        self.w.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.borrow_mut().flush()
    }
}

#[macro_export]
macro_rules! write_errln {
    ($w:expr, $($arg:tt)*) => {{
        let _ = writeln!(($w), $($arg)*);
        let _ = ($w).flush();
    }};
}

lazy_static! {
    static ref VERBOSE: Mutex<bool> = Mutex::new(false);
}

pub fn set_verbose(verb: bool) {
    let mut var = VERBOSE.lock().unwrap();
    *var = verb;
}

pub fn get_verbose() -> bool { VERBOSE.lock().unwrap().clone() }

// tt - TokenTree
// ($($arg:tt)*) Variable number of tts
#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => {{
        if crate::log::get_verbose() {
            print!($($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! verboseln {
    ($($arg:tt)*) => {{
        if crate::log::get_verbose() {
            println!($($arg)*);
        }
    }};
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{StringBuffer, WriteHandle};

    #[test]
    fn test_macros() {
        verbose!("something {}", 1);
        verboseln!("something {}", 1);
    }

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
        write_errln!(handle, "Another {}", "error");
        assert_eq!(buff.borrow().as_str(), "Some text 1\nAnother error\n");
    }
}