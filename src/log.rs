use std::sync::Mutex;

use lazy_static::lazy_static;

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

pub fn get_verbose() -> bool {
    VERBOSE.lock().unwrap().clone()
}

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

    use crate::util::rw::WriteHandle;

    #[test]
    fn test_macros() {
        verbose!("something {}", 1);
        verboseln!("something {}", 1);
    }

    #[test]
    fn test_write_errln() {
        let (mut handle, buff) = WriteHandle::string_buff_write_handle();
        write_errln!(handle, "Another {}", "error");
        assert_eq!(buff.borrow().as_str(), "Another error\n");
    }
}
