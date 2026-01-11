extern crate glob;
extern crate gsparser;

use glob::glob;
use gsparser::{
    mdl::{MdlFile, MdlHeader},
    steam::get_half_life_steam_install_path,
    util::resolve_null_terminated_string,
};
use std::{
    cmp::Ordering,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

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

    let mdl_files = collect_mdls(&game_root);

    let mut model_infos = Vec::new();
    for mdl_path in &mdl_files {
        let mdl_name = mdl_path.file_stem().unwrap().to_str().unwrap();
        let header = read_header(mdl_path);
        if header.is_idst() && header.has_sequences() {
            if let Ok(file) = MdlFile::open(mdl_path) {
                let mut sequence_infos = Vec::new();
                for sequence in &file.animation_sequences {
                    let sequence_name = resolve_null_terminated_string(&sequence.name);
                    let activity = sequence.activity;
                    if activity > 0 {
                        sequence_infos.push((sequence_name.to_string(), activity));
                    }
                }
                if sequence_infos.len() > 0 {
                    sequence_infos
                        .sort_by(|(name1, _), (name2, _)| -> Ordering { name1.cmp(name2) });
                    model_infos.push((mdl_name.to_owned(), sequence_infos));
                }
            }
        }
    }

    model_infos.sort_by(|(name1, _), (name2, _)| -> Ordering { name1.cmp(name2) });

    println!();
    for (model_name, sequence_infos) in model_infos {
        println!("{}:", model_name);
        for (sequence_name, activity) in sequence_infos {
            println!("  {}  -  {}", sequence_name, activity);
        }
    }
}

fn collect_mdls(path: &PathBuf) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let search = format!("{}/models/*.mdl", path.display());
    let bsps = glob(&search).unwrap();
    for bsp in bsps {
        let bsp = bsp.unwrap();
        paths.push(bsp);
    }
    paths
}

fn read_header<P: AsRef<Path>>(path: P) -> MdlHeader {
    let file = File::open(path).unwrap();
    let mut file = BufReader::new(file);
    let header: MdlHeader = bincode::deserialize_from(&mut file).unwrap();
    header
}
