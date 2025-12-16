use std::path::PathBuf;

use crate::resource::parse_resource_item;

pub fn get_half_life_steam_install_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use windows_registry::*;

        #[cfg(target_pointer_width = "32")]
        const STEAM_KEY_PATH: &str = r#"SOFTWARE\Valve\Steam"#;

        #[cfg(target_pointer_width = "64")]
        const STEAM_KEY_PATH: &str = r#"SOFTWARE\Wow6432Node\Valve\Steam"#;

        if let Ok(key) = LOCAL_MACHINE.open(STEAM_KEY_PATH) {
            if let Ok(value) = key.get_string("InstallPath") {
                //println!("STEAM PATH: {}", value);

                // Find libraryfolders.vdf
                let library_info_path = {
                    let mut path = PathBuf::from(value);
                    path.push("config/libraryfolders.vdf");
                    path
                };
                if let Ok(library_info_text) = std::fs::read_to_string(library_info_path) {
                    let mut lines = library_info_text.lines();
                    if let Some(root_item) = parse_resource_item(&mut lines) {
                        //println!("{:#?}", root_item);
                        if root_item.key == "libraryfolders" {
                            // Each child should be a library folder
                            let libraries = root_item.value.as_collection()?;
                            for library in &libraries.0 {
                                let library_items = library.value.as_collection()?;

                                // Get the path
                                if let Some(folder_path) = library_items.get("path") {
                                    let folder_path = folder_path.value.as_single()?;

                                    let apps = library_items.get("apps")?;
                                    let app_items = apps.value.as_collection()?;
                                    for app in &app_items.0 {
                                        // Half-Life's steamid is 70
                                        if app.key == "70" {
                                            // Fixup our path
                                            let path = folder_path.replace("\\\\", "\\");
                                            let mut path = PathBuf::from(path);
                                            path.push("steamapps/common/Half-Life/valve");
                                            return Some(path);
                                        }
                                    }
                                }
                            }
                        }
                    };
                }
            }
        }

        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}
