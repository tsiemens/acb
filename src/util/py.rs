use std::{
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};

use super::basic::SError;

// Generated by build.rs
include!(concat!(env!("OUT_DIR"), "/venv_constants.rs"));

pub fn run_python_script_file(
    script_path: &std::path::Path,
    args: Vec<String>,
) -> Result<String, SError> {
    let child = Command::new(PYTHON_BIN_PATH)
        .arg(script_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).map_err(|e| e.to_string())?)
    } else {
        Err(format!(
            "Python script {} failed: {}",
            script_path.display(),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn run_python_script(script: &str) -> Result<String, SError> {
    // Execute the Python script
    let child = Command::new(PYTHON_BIN_PATH)
        .arg("-c")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).map_err(|e| e.to_string())?)
    } else {
        Err(format!(
            "Python script execution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn get_python_script_dir() -> PathBuf {
    // Path is like <repo_root>/target/{debug|release}/build/acb-xxxxxx/out/venv
    // We want to get to the repo root.
    let repo_root_path =
        PathBuf::from_str(VENV_PATH).unwrap().join("../../../../../..");
    let py_dir = repo_root_path.join("py");
    py_dir.canonicalize().unwrap()
}

pub fn get_python_version() -> Result<String, SError> {
    let script = r#"
import sys
print(f"Python version: {sys.version}")
print("Hello from Python!")
"#;

    match run_python_script(script) {
        Ok(output) => Ok(output),
        Err(e) => Err(e),
    }
}