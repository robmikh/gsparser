use std::path::PathBuf;

pub struct BackgroundLayout {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<BackgroundTile>,
}

pub struct BackgroundTile {
    pub path: PathBuf,
    pub layout: TileLayout,
}

pub enum TileLayout {
    Fill { x: u32, y: u32 },
}

impl BackgroundLayout {
    pub fn parse(layout_text: &str) -> Self {
        let mut lines = layout_text.lines();

        // First, look for the resolution
        let mut width = 0;
        let mut height = 0;
        for line in &mut lines {
            let line = line.trim();
            if line.starts_with("//") || line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let command = parts.next().unwrap();
            if command == "resolution" {
                width = parts.next().unwrap().parse().unwrap();
                height = parts.next().unwrap().parse().unwrap();
                break;
            } else {
                continue;
            }
        }

        assert!(width != 0 && height != 0);

        // Read tile info
        let mut tiles = Vec::new();
        for line in lines {
            let line = line.trim();
            if line.starts_with("//") || line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let path = parts.next().unwrap();
            let layout = parts.next().unwrap();
            let layout = match layout {
                "fit" => {
                    let x: u32 = parts.next().unwrap().parse().unwrap();
                    let y: u32 = parts.next().unwrap().parse().unwrap();
                    TileLayout::Fill { x, y }
                }
                _ => todo!("Layout \"{}\" not implemented!", layout),
            };

            tiles.push(BackgroundTile {
                path: PathBuf::from(path),
                layout,
            })
        }

        Self {
            width,
            height,
            tiles,
        }
    }
}
