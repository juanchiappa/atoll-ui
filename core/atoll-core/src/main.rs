//! Punto de entrada del Core de Atoll.

mod island;

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use atoll_config::Config;
use atoll_platform::native_backend;

use crate::island::{island_config_from, load_or_create_config, IslandPainter};

/// Nombre de la subcarpeta dentro de `%APPDATA%`.
const APP_DIR_NAME: &str = "Atoll";

/// Nombre del archivo de configuración.
const CONFIG_FILE_NAME: &str = "config.ini";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("atoll: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Resuelve la carpeta `%APPDATA%\Atoll` y la ruta `%APPDATA%\Atoll\config.ini`
/// a partir del valor de `APPDATA`.
///
/// Si `APPDATA` no está definido o está vacío, devuelve `None`.
#[must_use]
fn config_paths_from_appdata(appdata: Option<OsString>) -> Option<(PathBuf, PathBuf)> {
    let base = appdata.filter(|value| !value.is_empty())?;
    let dir = PathBuf::from(base).join(APP_DIR_NAME);
    let file = dir.join(CONFIG_FILE_NAME);
    Some((dir, file))
}

/// Abre la isla con la configuración de `%APPDATA%\Atoll\config.ini` (o la por defecto)
/// y espera a que se cierre.
fn run() -> Result<(), Box<dyn Error>> {
    let (config, config_dir, config_file) = match config_paths_from_appdata(env::var_os("APPDATA"))
    {
        Some((dir, file)) => {
            let loaded = load_or_create_config(&dir, &file);
            (loaded, Some(dir), Some(file))
        }
        None => (Config::default(), None, None),
    };

    let island_config = island_config_from(&config, config_dir.clone());
    let mut backend = native_backend()?;
    let painter = IslandPainter::from_config(config, config_dir, config_file)?;
    backend.run(&island_config, Box::new(painter))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::config_paths_from_appdata;

    #[test]
    fn resolves_appdata_config_paths_or_none_when_missing() {
        assert_eq!(config_paths_from_appdata(None), None);
        assert_eq!(config_paths_from_appdata(Some(OsString::from(""))), None);

        let base = OsString::from("C:\\Users\\juan\\AppData\\Roaming");
        let (dir, file) = config_paths_from_appdata(Some(base)).expect("resuelve ruta");
        assert_eq!(
            dir,
            PathBuf::from("C:\\Users\\juan\\AppData\\Roaming").join("Atoll")
        );
        assert_eq!(file, dir.join("config.ini"));
    }
}
