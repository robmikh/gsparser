extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::{
    bsp::{BspEntity, BspReader},
    mdl::null_terminated_bytes_to_str,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::PathBuf,
};

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
        let mut model_references = HashMap::<usize, Vec<usize>>::new();
        for (entity_index, entity) in entities.iter().enumerate() {
            if let Some(model_str) = entity.0.get("model") {
                if model_str.starts_with("*") {
                    let model_index_str = &model_str[1..];
                    let model_index: usize = model_index_str.parse().unwrap();
                    if let Some(entities) = model_references.get_mut(&model_index) {
                        entities.push(entity_index);
                    } else {
                        let entities = vec![entity_index];
                        model_references.insert(model_index, entities);
                    }
                }
            }
        }

        // Remove models with only one entity reference
        let mut model_infos = Vec::new();
        let mut num_entities = 0;
        for (model_index, mut entities) in model_references {
            if entities.len() > 1 {
                num_entities += entities.len();
                entities.sort();
                model_infos.push((model_index, entities));
            }
        }
        model_infos
            .sort_by(|(model_index_1, _), (model_index_2, _)| model_index_1.cmp(model_index_2));

        if !model_infos.is_empty() {
            map_info.push((map_name.to_string(), model_infos, num_entities));
        }
    }

    // Report
    if map_info.is_empty() {
        println!("None!");
    } else {
        map_info.sort_by(|(name_1, _, _), (name_2, _, _)| name_1.cmp(name_2));

        for (name, model_references, num_entities) in map_info {
            println!("{} - {} entities share a model", name, num_entities);
            println!("    Model    Entities");
            for (model_index, entities) in model_references {
                println!("    {:>5}    {:?}", model_index, entities);
            }
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
