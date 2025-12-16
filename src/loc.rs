use std::{collections::HashMap, path::PathBuf};

use super::resource::{ParseResourceValue, parse_resource_item};

pub type UILocalization<'a> = HashMap<&'a str, &'a str>;

pub fn load_ui_loc_english_text(half_life_path: &PathBuf) -> String {
    // We also need to parse the localization file
    let loc_path = {
        let mut path = half_life_path.clone();
        path.push("resource/gameui_english.txt");
        path
    };
    // The data seems to be encoded as UTF-16...
    //let loc_text = std::fs::read_to_string(&loc_path).unwrap();
    let loc_bytes = std::fs::read(&loc_path).unwrap();
    // Shhhh...
    let loc_wide_bytes = unsafe {
        let ptr = loc_bytes.as_ptr();
        let len = loc_bytes.len() / 2;
        std::slice::from_raw_parts(ptr as *const u16, len)
    };
    let loc_text = String::from_utf16(loc_wide_bytes).unwrap();
    // Strip out BOM
    let loc_text = loc_text.trim_start_matches('\u{feff}');
    // TODO: Avoid extra alloc...
    loc_text.to_string()
}

pub fn parse_ui_loc<'a>(loc_text: &'a str) -> UILocalization<'a> {
    let mut loc_lines = loc_text.lines();
    let loc = parse_resource_item(&mut loc_lines).unwrap();
    //println!("{:#?}", loc);

    // Build a dicitonary from the localization data
    let localized_strings = {
        let mut strings = HashMap::new();

        // The top level should be "lang"
        assert!(loc.key == "lang");

        // All the strings should be in the "Tokens" item
        let root_items = loc.value.as_collection().unwrap();
        let tokens = root_items.get("Tokens").unwrap();
        let token_items = tokens.value.as_collection().unwrap();

        // Build the map
        for item in &token_items.0 {
            if let Some(value) = item.value.as_single() {
                strings.insert(item.key, value);
            }
        }

        strings
    };

    localized_strings
}

#[derive(Debug)]
pub enum LocalizedString {
    NonLocalized(String),
    Localized(String),
}

impl LocalizedString {
    pub fn new(value: &str) -> Self {
        if value.starts_with('#') {
            Self::Localized(value.trim_start_matches('#').to_string())
        } else {
            Self::NonLocalized(value.to_string())
        }
    }

    pub fn decode(&self, ui_loc: &UILocalization) -> String {
        match self {
            LocalizedString::NonLocalized(value) => value.clone(),
            LocalizedString::Localized(value) => ui_loc.get(value.as_str()).unwrap().to_string(),
        }
    }
}

impl ParseResourceValue for LocalizedString {
    fn parse(resource: &super::resource::ResourceItem) -> Option<Self> {
        let value = resource.value.as_single()?;
        Some(Self::new(value))
    }

    fn default_value() -> Option<Self> {
        None
    }
}
