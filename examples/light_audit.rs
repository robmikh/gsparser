extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::{
    bsp::{BspEntity, BspReader},
    steam::get_half_life_steam_install_path,
    util::resolve_map_entity_string,
};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let (game_root, output_path) = if args.len() == 1 {
        // Infer the game root via Steam
        let game_root =
            get_half_life_steam_install_path().expect("Failed to find Half-Life install location!");
        // Arg is the output path
        let output_path = PathBuf::from(args.get(0).unwrap());
        (game_root, output_path)
    } else if args.len() == 2 {
        let game_root = PathBuf::from(args.get(0).unwrap());
        let output_path = PathBuf::from(args.get(1).unwrap());
        (game_root, output_path)
    } else {
        panic!("Expected file output path!");
    };

    let maps = collect_maps(&game_root);

    let mut light_types = HashMap::<String, (usize, HashSet<String>)>::new();
    let mut max_light_styles = 0;
    for bsp_path in &maps {
        let map_name = bsp_path.file_stem().unwrap().to_str().unwrap();
        let data = std::fs::read(bsp_path).unwrap();
        let reader = BspReader::read(data);

        let entity_string = resolve_map_entity_string(&reader);
        let entities = BspEntity::parse_entities(&entity_string);
        let mut seen_light_styles = HashSet::new();
        for entity in &entities {
            let entity_type = *entity.0.get("classname").unwrap();
            if entity_type.starts_with("light") {
                if let Some(style) = entity.0.get("style") {
                    if *style != "0" {
                        if !seen_light_styles.contains(*style) {
                            seen_light_styles.insert(style.to_owned());
                        }

                        if let Some((count, seen_maps)) = light_types.get_mut(entity_type) {
                            *count += 1;
                            if !seen_maps.contains(map_name) {
                                seen_maps.insert(map_name.to_owned());
                            }
                        } else {
                            let mut seen_maps = HashSet::new();
                            seen_maps.insert(map_name.to_owned());
                            light_types.insert(entity_type.to_owned(), (1, seen_maps));
                        }
                    }
                }
            }
        }
        max_light_styles = max_light_styles.max(seen_light_styles.len());
    }

    let mut sorted_types = light_types.into_iter().collect::<Vec<_>>();
    sorted_types.sort_by(|(ty1, (count1, _)), (ty2, (count2, _))| -> Ordering {
        let ordering = count2.cmp(&count1);
        if ordering == Ordering::Equal {
            ty1.cmp(&ty2)
        } else {
            ordering
        }
    });

    let mut lights_string = String::new();
    writeln!(&mut lights_string, "Max light styles: {}", max_light_styles).unwrap();
    writeln!(&mut lights_string, "").unwrap();

    let mut maps_string = String::new();
    for (light_type, (count, seen_maps)) in sorted_types {
        writeln!(&mut lights_string, "{:<16} -  {}", light_type, count).unwrap();

        writeln!(&mut maps_string, "{}:", light_type).unwrap();
        let mut seen_maps: Vec<_> = seen_maps.iter().collect();
        seen_maps.sort();
        for map in seen_maps {
            writeln!(&mut maps_string, "  {}", map).unwrap();
        }
    }
    writeln!(&mut lights_string, "").unwrap();
    write!(&mut lights_string, "{}", maps_string).unwrap();

    std::fs::write(&output_path, lights_string).unwrap();
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
