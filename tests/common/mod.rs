use std::{fs, path::PathBuf};

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
    pub path: PathBuf
}

impl NonAutoCreatingTestDir {
    pub fn new() -> NonAutoCreatingTestDir {
        NonAutoCreatingTestDir{path: test_temp_dir_path()}
    }
}

fn cleanup_test_dir(path: &PathBuf) {
    if path.exists() {
        let skip_env_var = "SKIP_TEMP_DIR_CLEANUP_ON_FAIL";
        let skip_del_on_fail = acb::util::sys::env_var_non_empty(skip_env_var);

        if std::thread::panicking() && skip_del_on_fail {
            println!("cleanup_test_dir: panicking. Skipping remove of {}",
                     path.to_str().unwrap());
        } else {
            println!("cleanup_test_dir: removing {}. To skip cleanup, set {}",
                     path.to_str().unwrap(), skip_env_var);
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