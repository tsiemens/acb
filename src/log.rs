use std::{fmt::Write, io, sync::Mutex};

use lazy_static::lazy_static;

use crate::util::rc::{RcRefCell, RcRefCellT};


lazy_static! {
    static ref DEBUG_TRACING_ENABLED: Mutex<Option<bool>> = Mutex::new(None);
}

fn set_debug_tracing_enabled(enabled: bool) {
    let mut setting = DEBUG_TRACING_ENABLED.lock().unwrap();
    *setting = Some(enabled);
}

pub fn get_debug_tracing_enabled() -> bool {
    match *DEBUG_TRACING_ENABLED.lock().unwrap() {
        Some(enabled) => enabled,
        None => false,
    }
}

fn load_trace_setting() {
    set_debug_tracing_enabled(
        crate::util::sys::env_var_non_empty("TRACE"));
}

fn maybe_load_trace_setting() {
    if DEBUG_TRACING_ENABLED.lock().unwrap().is_none() {
        load_trace_setting()
    }
}

pub fn setup_tracing() {
    maybe_load_trace_setting();
}

// TODO replace this with the more sophisticated tracing crate
#[macro_export]
macro_rules! acb_debug {
    ($($arg:tt)*) => {{
        if crate::log::get_debug_tracing_enabled() {
            eprint!("DEBUG: ");
            eprintln!($($arg)*);
        }
    }};
}

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
        acb_debug!("WriteHandle::write {}", { let mut b = StringBuffer::new(); let _ = b.write(buf); b }.as_str() );
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