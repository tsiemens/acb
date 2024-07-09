use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Create a convenience wrapper for the python binary, that will run in the venv.
/// Mostly for debugging.
#[cfg(unix)]
fn make_python_bin_wrapper(target_dir: &Path, venv_path: &Path, python_path: &Path)
-> Result<(), String> {

    let wrapper_path = Path::new(&target_dir).join("python");
    let python_wrapper_script = format!(
        r#"#!/usr/bin/env bash
{}
exec {} "$@"
"#,
        Path::new(&venv_path).join("bin/activate").display(),
        python_path.display()
    );

    fs::write(&wrapper_path, python_wrapper_script)
        .map_err(|e| format!("Error writing {:?}: {}", wrapper_path, e))?;
    let mut perms = fs::metadata(&wrapper_path)
        .map_err(|e| format!("Error creating fs::metadata for {:?}: {}",
                                wrapper_path, e))?.permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o700);
    fs::set_permissions(&wrapper_path, perms)
        .map_err(|e| format!("Error setting permissions on {:?}: {}",
                             wrapper_path, e))?;
    Ok(())
}

fn install_python_venv() {
    let emit_verbose_warnings = env::var("BUILD_RS_WARN_VERBOSE").is_ok();

    let out_dir = env::var("OUT_DIR").unwrap();
    let venv_path = Path::new(&out_dir).join("venv");
    let target_dir = Path::new(&out_dir).join("../../..");
    if emit_verbose_warnings {
        println!("cargo::warning=Installing venv in {:?}", venv_path);
    }

    // Create virtualenv
    Command::new("python3")
        .args(&["-m", "venv", venv_path.to_str().unwrap()])
        .status()
        .expect("Failed to create virtualenv");

    // Write requirements.txt
    let requirements = "pypdf==4.2.0\n";
    fs::write(Path::new(&out_dir).join("requirements.txt"), requirements)
        .expect("Failed to write requirements.txt");

    // Get path to pip in the new virtualenv
    let (pip_path, python_path) = if cfg!(windows) {
        (venv_path.join("Scripts").join("pip.exe"),
         venv_path.join("Scripts").join("python.exe"))
    } else {
        (venv_path.join("bin").join("pip"),
         venv_path.join("bin").join("python"))
    };

    // Run pip install
    Command::new(pip_path.clone())
        .args(&[
            "install",
            "--require-virtualenv",
            "-r",
            Path::new(&out_dir).join("requirements.txt").to_str().unwrap(),
        ])
        .status()
        .expect("Failed to run pip install");


    if cfg!(unix) {
        if let Err(e) = make_python_bin_wrapper(
            &target_dir, &venv_path, &python_path) {
            // This is not critical.
            println!("cargo::warning={}", e);
        }
    }

    // Generate Rust file with constants
    let venv_constants = format!(
        r#"
pub const VENV_PATH: &str = "{}";
pub const PIP_PATH: &str = "{}";
pub const PYTHON_BIN_PATH: &str = "{}";
"#,
        venv_path.display(),
        pip_path.display(),
        python_path.display()
    );

    fs::write(
        Path::new(&out_dir).join("venv_constants.rs"),
        venv_constants,
    )
    .expect("Failed to write venv_constants.rs");
}

fn main() {
    install_python_venv();
    println!("cargo::rerun-if-changed=build.rs");
}