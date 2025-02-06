use std::path::Path;
use crate::error::Error;

pub fn validate_file_permissions(path: &Path) -> Result<(), Error> {
    let metadata = path.metadata()?;

    if !metadata.is_file() {
        return Err(Error::Security(format!("Path is not a file: {}", path.display())));
    }

    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        if mode & 0o7777 != 0o600 {
            return Err(Error::Security(format!(
                "Insecure permissions for {}: {:o}",
                path.display(),
                mode & 0o7777
            )));
        }
    }

    Ok(())
}

pub fn load_pem(path: &Path) -> Result<Vec<u8>, Error> {
    // validate_file_permissions(path)?;
    std::fs::read(path).map_err(Into::into)
}