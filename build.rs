use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Create a convenience wrapper for the python binary, that will run in the venv.
/// Mostly for debugging.
#[cfg(unix)]
fn make_python_bin_wrapper(
    target_dir: &Path,
    venv_path: &Path,
    python_path: &Path,
) -> Result<(), String> {
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
        .map_err(|e| {
            format!("Error creating fs::metadata for {:?}: {}", wrapper_path, e)
        })?
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o700);
    fs::set_permissions(&wrapper_path, perms).map_err(|e| {
        format!("Error setting permissions on {:?}: {}", wrapper_path, e)
    })?;
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
        (
            venv_path.join("Scripts").join("pip.exe"),
            venv_path.join("Scripts").join("python.exe"),
        )
    } else {
        (
            venv_path.join("bin").join("pip"),
            venv_path.join("bin").join("python"),
        )
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
        if let Err(e) =
            make_python_bin_wrapper(&target_dir, &venv_path, &python_path)
        {
            // This is not critical.
            println!("cargo::warning={}", e);
        }
    }

    write_venv_constants(
        &out_dir,
        &venv_path.display().to_string(),
        &pip_path.display().to_string(),
        &python_path.display().to_string(),
    );
}

/// Get the repo root from OUT_DIR.
/// OUT_DIR is like <repo_root>/target/{debug|release}/build/acb-xxxxxx/out
fn get_repo_root(out_dir: &str) -> std::path::PathBuf {
    // OUT_DIR is like <repo_root>/target/{debug|release}/build/acb-xxxxxx/out
    Path::new(out_dir)
        .join("../../../../..")
        .canonicalize()
        .expect("Failed to canonicalize repo root path")
}

/// Resolve the node binary path using fnm + .node-version.
/// Returns None if fnm is not available.
fn resolve_node_bin_path(repo_root: &Path) -> Option<std::path::PathBuf> {
    let home = env::var("HOME").ok()?;
    let fnm_path = Path::new(&home).join(".cargo/bin/fnm");

    // Check if fnm exists either on PATH or in ~/.cargo/bin
    let fnm_cmd = if fnm_path.exists() {
        fnm_path.to_str().unwrap().to_string()
    } else {
        // Check PATH
        let check = Command::new("which")
            .arg("fnm")
            .output()
            .ok()?;
        if !check.status.success() {
            return None;
        }
        "fnm".to_string()
    };

    let www_dir = repo_root.join("www");

    // Use fnm to get the node binary path. We need to:
    // 1. eval fnm env to set up the PATH
    // 2. cd to www/ so fnm reads .node-version
    // 3. fnm use to install/activate the version
    // 4. output the real (resolved) node binary path
    //    (fnm uses ephemeral symlinks in /run/ that don't persist)
    let script = format!(
        r#"
export PATH="$HOME/.cargo/bin:$PATH"
eval "$({fnm_cmd} env --shell bash)"
cd {www_dir} && {fnm_cmd} use --install-if-missing --silent-if-unchanged
realpath "$(which node)"
"#,
        fnm_cmd = fnm_cmd,
        www_dir = www_dir.display(),
    );

    let output = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .ok()?;

    if output.status.success() {
        let node_path = String::from_utf8(output.stdout).ok()?.trim().to_string();
        if !node_path.is_empty() {
            Some(std::path::PathBuf::from(node_path))
        } else {
            None
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("cargo::warning=fnm node resolution failed: {}", stderr);
        None
    }
}

fn install_node_modules(emit_verbose_warnings: bool) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let repo_root = get_repo_root(&out_dir);

    let node_bin_path = match resolve_node_bin_path(&repo_root) {
        Some(p) => p,
        None => {
            println!(
                "cargo::warning=fnm/node not available. \
                 Node.js PDF reader (pdfjs-dist) will not be functional. \
                 Run 'cargo install fnm' to enable it."
            );
            write_node_constants(&out_dir, None, None, None);
            return;
        }
    };

    if emit_verbose_warnings {
        println!(
            "cargo::warning=Resolved node binary: {:?}",
            node_bin_path
        );
    }

    // Determine npm path (sibling of node)
    let npm_script = repo_root.join("www/scripts/npm");

    // Set up node_modules in OUT_DIR
    let node_env_dir = Path::new(&out_dir).join("node_env");
    fs::create_dir_all(&node_env_dir).expect("Failed to create node_env directory");

    // Write package.json
    let pdfjs_version = "^5.5.207";

    // If www/package.json also has pdfjs-dist, assert the versions match.
    let www_package_json_path = repo_root.join("www/package.json");
    if let Ok(www_pkg_contents) = fs::read_to_string(&www_package_json_path) {
        // Simple extraction: look for "pdfjs-dist": "<version>" in the JSON.
        // We avoid pulling in a JSON parser dep just for build.rs.
        if let Some(pos) = www_pkg_contents.find("\"pdfjs-dist\"") {
            // Find the version string value after the key
            let after_key = &www_pkg_contents[pos..];
            if let (Some(colon), _) = (after_key.find(':'), ()) {
                let after_colon = after_key[colon + 1..].trim_start();
                if after_colon.starts_with('"') {
                    let version_start = 1; // skip opening quote
                    if let Some(end_quote) =
                        after_colon[version_start..].find('"')
                    {
                        let www_version =
                            &after_colon[version_start..version_start + end_quote];
                        assert!(
                            www_version == pdfjs_version,
                            "pdfjs-dist version mismatch: build.rs has \
                             \"{pdfjs_version}\" but www/package.json has \
                             \"{www_version}\". Please keep them in sync."
                        );
                    }
                }
            }
        }
    }

    let package_json = format!(
        r#"{{
  "name": "acb-node-env",
  "private": true,
  "dependencies": {{
    "pdfjs-dist": "{pdfjs_version}"
  }}
}}
"#
    );
    fs::write(node_env_dir.join("package.json"), package_json)
        .expect("Failed to write package.json");

    // Run npm install using the www/scripts/npm wrapper
    let status = Command::new(npm_script.to_str().unwrap())
        .args(&["install", "--prefix", node_env_dir.to_str().unwrap()])
        .status()
        .expect("Failed to run npm install for node_env");

    if !status.success() {
        println!("cargo::warning=npm install for pdfjs-dist failed. Node.js PDF reader will not be functional.");
        write_node_constants(&out_dir, None, None, None);
        return;
    }

    let node_modules_path = node_env_dir.join("node_modules");

    write_node_constants(
        &out_dir,
        Some(&node_bin_path.display().to_string()),
        Some(&node_modules_path.display().to_string()),
        Some(&repo_root.display().to_string()),
    );
}

fn write_venv_constants(out_dir: &str, venv_path: &str, pip_path: &str, python_bin_path: &str) {
    let content = format!(
        r#"pub const VENV_PATH: &str = "{venv_path}";
pub const PIP_PATH: &str = "{pip_path}";
pub const PYTHON_BIN_PATH: &str = "{python_bin_path}";
"#
    );
    fs::write(Path::new(out_dir).join("venv_constants.rs"), content)
        .expect("Failed to write venv_constants.rs");
}

fn write_node_constants(
    out_dir: &str,
    node_bin_path: Option<&str>,
    node_modules_path: Option<&str>,
    node_repo_root: Option<&str>,
) {
    let fmt = |v: Option<&str>| match v {
        Some(s) => format!("Some(\"{s}\")"),
        None => "None".to_string(),
    };
    let content = format!(
        r#"pub const NODE_BIN_PATH: Option<&str> = {};
pub const NODE_MODULES_PATH: Option<&str> = {};
pub const NODE_REPO_ROOT: Option<&str> = {};
"#,
        fmt(node_bin_path),
        fmt(node_modules_path),
        fmt(node_repo_root),
    );
    fs::write(Path::new(out_dir).join("node_constants.rs"), content)
        .expect("Failed to write node_constants.rs");
}

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let emit_verbose_warnings = env::var("BUILD_RS_WARN_VERBOSE").is_ok();

    // Python venv and node modules are only needed for native builds, not WASM.
    if !target.contains("wasm") {
        install_python_venv();
        install_node_modules(emit_verbose_warnings);
    }

    println!("cargo::rerun-if-changed=build.rs");
}
