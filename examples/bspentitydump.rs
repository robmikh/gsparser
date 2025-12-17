extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::{bsp::BspReader, util::resolve_map_entity_string};
use std::path::PathBuf;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let game_root = args.get(0).unwrap();
    let output_path = args.get(1).unwrap();

    let maps = collect_maps(&game_root);

    // Ensure output path
    let mut output_path = PathBuf::from(output_path);
    if !output_path.exists() {
        std::fs::create_dir(&output_path).expect("Failed to make output directory!");
    }

    output_path.push("dummy");
    for bsp_path in &maps {
        let map_name = bsp_path.file_stem().unwrap().to_str().unwrap();
        let data = std::fs::read(bsp_path).unwrap();
        let reader = BspReader::read(data);

        let entity_string = resolve_map_entity_string(&reader);
        output_path.set_file_name(format!("{}.txt", map_name));
        std::fs::write(&output_path, entity_string.to_string()).unwrap();
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
