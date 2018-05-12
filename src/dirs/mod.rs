use std::path::Path;
use std::{env,fs};
use app_dirs::{AppInfo, get_app_root, AppDataType};
use std::ffi::OsString;

const AUTHOR: &'static str = "mmacedoeu";
const PRODUCT: &'static str = "swapi";

#[derive(Debug, PartialEq)]
pub struct Directories {
    pub base: String,
    pub db: String,
}

impl Default for Directories {
    fn default() -> Self {
        let data_dir = default_data_path();
        let base = replace_home(&data_dir, "$BASE");
        Directories {
            db: db_root_path(&base).into_string().unwrap(),
            base: base,            
        }
    }
}

impl Directories {
    pub fn create_dirs(&self)
                       -> Result<(), String> {
        fs::create_dir_all(&self.base)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn default_data_path() -> String {
    let app_info = AppInfo {
        name: PRODUCT,
        author: AUTHOR,
    };
    get_app_root(AppDataType::UserData, &app_info)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "$HOME/.swapi".to_owned())
}

/// Replaces `$HOME` str with home directory path.
pub fn replace_home(base: &str, arg: &str) -> String {
    // the $HOME directory on mac os should be `~/Library` or `~/Library/Application Support`
    let r = arg.replace("$HOME", env::home_dir().unwrap().to_str().unwrap());
    let r = r.replace("$BASE", base);
    r.replace("/", &::std::path::MAIN_SEPARATOR.to_string())
}

pub fn db_root_path(base: &str) -> OsString {
    let mut dir = Path::new(base).to_path_buf();
    dir.push("db");
    dir.into_os_string()
}
