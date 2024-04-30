use std::{fs, io, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};

pub type Error = String;

fn mk_writable_dir(dirpath: &Path) -> io::Result<()> {
    fs::create_dir_all(dirpath)?;

    let mut perms = fs::metadata(dirpath)?.permissions();
    perms.set_readonly(false);
    perms.set_mode(0o700);
    fs::set_permissions(dirpath, perms)
}

// Returns a path like $HOME/.acb/, and ensures that ~/.acb/ exists and is writable.
pub fn home_dir_path() -> Result<PathBuf, Error> {
    let home_dir_opt = dirs::home_dir();
    let home_dir = match home_dir_opt {
        Some(d) => d,
        // None => return Err(Error::from("Unable to determine home directory")),
        None => return Err(Error::from("Unable to determine home directory")),
    };

    let acb_dir_path = home_dir.join(".acb");
    mk_writable_dir(&acb_dir_path).map_err(
        |e| Error::from(e.to_string()))?;
    Ok(acb_dir_path)
}

// With a file name (eg. foo.txt), returns a path like $HOME/.acb/foo.txt,
// and ensures that ~/.acb/ exists and is writable.
pub fn home_dir_file_path(fname: &Path) -> Result<PathBuf, Error> {
    let acb_dir_path = home_dir_path()?;
    Ok(acb_dir_path.join(fname))
}