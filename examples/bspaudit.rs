extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::bsp::BspReader;
use std::{env, path::PathBuf};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let path = args.get(0).unwrap();

    let maps = collect_maps(&path);

    print_max_num_textures(&maps);
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
