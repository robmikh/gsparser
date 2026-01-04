extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::{
    bsp::{BspEntity, BspReader},
    steam::get_half_life_steam_install_path,
    util::resolve_map_entity_string,
};
use std::path::PathBuf;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let game_root = if args.len() == 1 {
        let game_root = PathBuf::from(args.get(0).unwrap());
        game_root
    } else {
        // Infer the game root via Steam
        let game_root =
            get_half_life_steam_install_path().expect("Failed to find Half-Life install location!");
        game_root
    };

    let maps = collect_maps(&game_root);

    let mut total_monsters = 0;
    let mut maps_with_monsters = 0;
    let mut max_monsters = i32::MIN;
    let mut min_monsters = i32::MAX;
    for bsp_path in &maps {
        let data = std::fs::read(bsp_path).unwrap();
        let reader = BspReader::read(data);

        let entity_string = resolve_map_entity_string(&reader);
        let entities = BspEntity::parse_entities(&entity_string);
        let mut total_monsters_on_map = 0;
        for entity in &entities {
            let entity_type = *entity.0.get("classname").unwrap();
            if entity_type.starts_with("monster_") {
                total_monsters_on_map += 1;
            }
        }

        if total_monsters_on_map > 0 {
            maps_with_monsters += 1;

            max_monsters = max_monsters.max(total_monsters_on_map);
            min_monsters = min_monsters.min(total_monsters_on_map);
            total_monsters += total_monsters_on_map;
        }
    }

    let average_monsters_per_map = total_monsters as f32 / maps_with_monsters as f32;

    println!("Average monsters per map: {}", average_monsters_per_map);
    println!("Max monsters on map: {}", max_monsters);
    println!("Min monsters on map: {}", min_monsters);
}

fn collect_maps(path: &PathBuf) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let search = format!("{}/**/*.bsp", path.display());
    let bsps = glob(&search).unwrap();
    for bsp in bsps {
        let bsp = bsp.unwrap();
        paths.push(bsp);
    }
    paths
}
