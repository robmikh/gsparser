#[derive(Clone, Debug)]
pub struct SpriteInfoFile {
    pub infos: Vec<SpriteInfo>,
}

#[derive(Clone, Debug)]
pub struct SpriteInfo {
    pub name: String,
    pub resolution: u32,
    pub file_path: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl SpriteInfoFile {
    // TODO: Return real errors
    pub fn parse(text: &str) -> Self {
        let mut lines = text.lines();

        // First, get the number of sprites
        let num_sprites = {
            let mut num_sprites: usize = 0;
            for line in &mut lines {
                let line = line.trim();
                if !line.is_empty() {
                    num_sprites = line.parse().unwrap();
                    break;
                }
            }
            num_sprites
        };

        // TODO: Return an error
        if num_sprites == 0 {
            panic!("No sprites found!");
        }

        // Parse sprites
        let mut infos = Vec::with_capacity(num_sprites);
        for line in &mut lines {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with("//") {
                if infos.len() >= num_sprites {
                    panic!("More text than expected!");
                }
                let sprite_info = SpriteInfo::parse(line);
                infos.push(sprite_info);
            }
        }

        Self { infos }
    }
}

impl SpriteInfo {
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        let mut parts = text.split_whitespace();
        let name = parts.next().unwrap().to_owned();
        let resolution_str = parts.next().unwrap().to_owned();
        let resolution: u32 = resolution_str.parse().unwrap();
        let file_path = parts.next().unwrap().to_owned();
        let x_str = parts.next().unwrap().to_owned();
        let x: u32 = x_str.parse().unwrap();
        let y_str = parts.next().unwrap().to_owned();
        let y: u32 = y_str.parse().unwrap();
        let width_str = parts.next().unwrap().to_owned();
        let width: u32 = width_str.parse().unwrap();
        let height_str = parts.next().unwrap().to_owned();
        let height: u32 = height_str.parse().unwrap();
        if parts.next().is_some() {
            panic!("More sprite properties than expected!");
        }
        Self {
            name,
            resolution,
            file_path,
            x,
            y,
            width,
            height,
        }
    }
}
