use std::collections::HashMap;

pub struct GameInfo<'a>(pub HashMap<&'a str, &'a str>);

impl<'a> GameInfo<'a> {
    pub fn parse(game_info_text: &'a str) -> Self {
        let mut map = HashMap::new();
        for line in game_info_text.lines() {
            let line = line.trim();
            if line.starts_with("//") || line.is_empty() {
                continue;
            }

            let (key, value) = line.split_once(' ').unwrap();
            map.insert(key, value.trim_matches('"'));
        }
        Self(map)
    }

    pub fn game_name(&self) -> &str {
        self.0.get("game").map(|x| *x).unwrap()
    }

    pub fn start_map(&self) -> &str {
        self.0.get("startmap").map(|x| *x).unwrap()
    }

    pub fn hd_background(&self) -> Option<bool> {
        self.0.get("hd_background").map(|x| -> bool {
            let value: i32 = x.parse().unwrap();
            value != 0
        })
    }
}
