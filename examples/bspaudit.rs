extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::{
    bsp::{BspEntity, BspReader},
    util::resolve_map_entity_string,
};
use std::{cmp::Ordering, collections::HashMap, env, path::PathBuf};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let path = args.get(0).unwrap();

    let maps = collect_maps(&path);

    print_max_num_textures(&maps);
    print_entity_types(&maps);
}

fn collect_maps(path: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let search = format!("{}/**/*.bsp", path);
    let bsps = glob(&search).unwrap();
    for bsp in bsps {
        let bsp = bsp.unwrap();
        paths.push(bsp);
    }
    paths
}

fn print_max_num_textures(paths: &[PathBuf]) {
    let mut max = u32::MIN;
    for bsp_path in paths {
        let data = std::fs::read(bsp_path).unwrap();
        let reader = BspReader::read(data);
        let textures = reader.read_textures_header();
        let num_textures = textures.num_textures;

        println!("bsp: {}  -  {}", bsp_path.display(), num_textures);
        max = max.max(num_textures);
    }
    println!("Max num textures: {}", max);
}

fn print_entity_types(paths: &[PathBuf]) {
    let mut entity_types = HashMap::<String, usize>::new();
    for bsp_path in paths {
        let data = std::fs::read(bsp_path).unwrap();
        let reader = BspReader::read(data);

        let entity_string = resolve_map_entity_string(&reader);
        let entities = BspEntity::parse_entities(&entity_string);
        for entity in &entities {
            let entity_type = *entity.0.get("classname").unwrap();

            if let Some(count) = entity_types.get_mut(entity_type) {
                *count += 1;
            } else {
                entity_types.insert(entity_type.to_owned(), 1);
            }
        }
    }

    let mut sorted_types = entity_types.into_iter().collect::<Vec<_>>();
    sorted_types.sort_by(|(ty1, count1), (ty2, count2)| -> Ordering {
        let ordering = count2.cmp(&count1);
        if ordering == Ordering::Equal {
            ty1.cmp(&ty2)
        } else {
            ordering
        }
    });

    for (entity_types, count) in sorted_types {
        println!("{:<16} -  {}", entity_types, count);
    }
}
