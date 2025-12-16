use std::path::PathBuf;

use crate::{
    loc::LocalizedString,
    resource::{ParseResource, parse_resource_item},
    resource_struct,
};

resource_struct!(MenuItemData {
    ("label") label : LocalizedString,
    ("command") command: String,
    ("HelpText") help: Option<LocalizedString>,
    ("OnlyInGame") only_in_game: Option<bool>,
    ("notsingle") not_single: Option<bool>,
    ("notmulti") not_multi: Option<bool>,
    ("notsteam") not_steam: Option<bool>,
});

pub fn parse_main_menu_items(half_life_path: &PathBuf) -> Vec<MenuItemData> {
    // Create the path for GameMenu.res
    let menu_resource_path = {
        let mut path = half_life_path.clone();
        path.push("resource/GameMenu.res");
        path
    };

    parse_menu_items(&menu_resource_path)
}

pub fn parse_menu_items(menu_resource_path: &PathBuf) -> Vec<MenuItemData> {
    let mut items = Vec::new();

    // First, we need to parse the resource file
    let resource_text = std::fs::read_to_string(&menu_resource_path).unwrap();
    let mut resource_lines = resource_text.lines();
    let game_menu = parse_resource_item(&mut resource_lines).unwrap();

    // The top level should be "GameMenu"
    assert!(game_menu.key == "GameMenu");
    let menu_items = game_menu.value.as_collection().unwrap();

    // Parse each menu item
    for item in &menu_items.0 {
        let menu_item = MenuItemData::parse(item).unwrap();
        items.push(menu_item);
    }

    items
}

pub struct StartGameMenuItemIterator<'a>(std::slice::Iter<'a, MenuItemData>);

impl<'a> StartGameMenuItemIterator<'a> {
    pub fn new(items: &'a [MenuItemData]) -> Self {
        Self(items.iter())
    }
}

impl<'a> Iterator for StartGameMenuItemIterator<'a> {
    type Item = &'a MenuItemData;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(menu_item) = self.0.next() {
            // Half-Life's main menu seems to only check "OnlyInGame" for filtering.
            // Additionally, pretend that we're the Steam version of the game. This
            // avoids including the "Change Game" menu item.
            if menu_item.only_in_game.unwrap_or(false) || menu_item.not_steam.unwrap_or(false) {
                continue;
            }
            return Some(menu_item);
        }
        None
    }
}
