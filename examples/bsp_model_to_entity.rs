extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::{
    bsp::{BspEntity, BspReader},
    mdl::null_terminated_bytes_to_str,
};
use std::{borrow::Cow, collections::HashSet, path::PathBuf};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let path = args.get(0).unwrap();

    let maps = collect_maps(&path);

    let mut map_info = Vec::new();
    for bsp_path in &maps {
        let map_name = bsp_path.file_stem().unwrap().to_str().unwrap();
        let data = std::fs::read(bsp_path).unwrap();
        let reader = BspReader::read(data);

        let entity_string = resolve_map_entity_string(&reader);

        // Go through every entity and collect model references
        let entities = BspEntity::parse_entities(&entity_string);
        let mut model_references = HashSet::new();
        for entity in entities {
            if let Some(model_str) = entity.0.get("model") {
                if model_str.starts_with("*") {
                    let index_str = &model_str[1..];
                    let index: usize = index_str.parse().unwrap();
                    model_references.insert(index);
                }
            }
        }

        // Go through each map (skip the first, that's never an entity)
        let mut models_with_no_entities = Vec::new();
        let models = reader.read_models();
        for model_index in 1..models.len() {
            if !model_references.contains(&model_index) {
                models_with_no_entities.push(model_index);
            }
        }

        if !models_with_no_entities.is_empty() {
            map_info.push((map_name.to_string(), models_with_no_entities));
        }
    }

    if map_info.is_empty() {
        println!("None!");
    } else {
        map_info.sort_by(|(name_1, _), (name_2, _)| name_1.cmp(name_2));

        // Report
        for (name, models_with_no_entities) in map_info {
            println!("{} - {} models", name, models_with_no_entities.len());
            println!("    {:?}", models_with_no_entities);
        }
    }
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

fn resolve_map_entity_string<'a>(reader: &'a BspReader) -> Cow<'a, str> {
    let entities_bytes = reader.read_entities();
    match null_terminated_bytes_to_str(entities_bytes) {
        Ok(entities) => Cow::Borrowed(entities),
        Err(error) => {
            println!("  WARNING: {:?}", error);
            let start = error.str_error.valid_up_to();
            let end = start + error.str_error.error_len().unwrap_or(1);
            println!("           error bytes: {:?}", &entities_bytes[start..end]);
            String::from_utf8_lossy(&entities_bytes[..error.end])
        }
    }
}
