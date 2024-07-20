use std::{fs, path::PathBuf};

// Note: Because this module is declared with mod in each integration test,
// functions here may appear as dead code while compiling each respective
// test, if the test doesn't use every single part of the lib. IDE may also
// get confused, depending on what context its compiling the file from.

#[allow(dead_code)]
fn test_temp_dir_path() -> PathBuf {
    let tmpdir = std::env::temp_dir();

    let make_file_path = |val| {
        let fname = format!("acb-test-{}", val);
        tmpdir.join(fname)
    };

    for val in 1..1000000 {
        let path = make_file_path(val);
        if !path.exists() {
            return path;
        }
    }
    panic!("Could not create temp directory path that does not already exist");
}

pub struct NonAutoCreatingTestDir {
    pub path: PathBuf,
}

impl NonAutoCreatingTestDir {
    #[allow(dead_code)]
    pub fn new() -> NonAutoCreatingTestDir {
        NonAutoCreatingTestDir {
            path: test_temp_dir_path(),
        }
    }
}

fn cleanup_test_dir(path: &PathBuf) {
    if path.exists() {
        let skip_env_var = "SKIP_TEMP_DIR_CLEANUP_ON_FAIL";
        let skip_del_on_fail = acb::util::sys::env_var_non_empty(skip_env_var);

        if std::thread::panicking() && skip_del_on_fail {
            println!(
                "cleanup_test_dir: panicking. Skipping remove of {}",
                path.to_str().unwrap()
            );
        } else {
            println!(
                "cleanup_test_dir: removing {}. To skip cleanup, set {}",
                path.to_str().unwrap(),
                skip_env_var
            );
            let _ = fs::remove_dir_all(path);
        }
    } else {
        println!("cleanup_test_dir: {} did not exist", path.to_str().unwrap());
    }
}

impl Drop for NonAutoCreatingTestDir {
    fn drop(&mut self) {
        cleanup_test_dir(&self.path);
    }
}

/// Used to run a sub-testlet, where test T is some kind of
/// function/lambda. Useful for iterating permutations.
/// Will nicely print out what the test name that failed is.
/// Will not result in deferred failures (the first testlet to fail
/// will block subsequent testlets from running).
#[allow(dead_code)]
pub fn run_test<T>(name: &str, test: T)
where
    T: FnOnce() + std::panic::UnwindSafe,
{
    println!("Running test: {}", name);
    let result = std::panic::catch_unwind(test);
    match result {
        Ok(_) => println!("{name} passed"),
        Err(e) => {
            panic!("{name} failed: {e:#?}");
        }
    }
}
