use std::fs::File;
use std::io::BufReader;
use std::{fmt::Write, path::PathBuf};

use gsparser::mdl::MdlHeader;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let game_root = args.get(0).unwrap();
    let output_path = args.get(1).unwrap();

    let models_path = {
        let mut path = PathBuf::from(game_root);
        path.push("models");
        path
    };

    let mut model_paths = Vec::new();
    for entry in std::fs::read_dir(models_path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let entry_path = entry.path();
            let extension = entry_path.extension().unwrap().to_str().unwrap();
            if extension == "mdl" {
                let file_stem = entry_path.file_stem().unwrap().to_str().unwrap();
                model_paths.push((file_stem.to_owned(), entry_path));
            }
        }
    }

    // Ensure output path
    let mut output_path = PathBuf::from(output_path);
    if !output_path.exists() {
        std::fs::create_dir(&output_path).expect("Failed to make output directory!");
    }

    // Print headers to output path
    output_path.push("dummy");
    for (stem, path) in model_paths {
        println!("{}...", stem);
        let mut text = String::new();

        let file = File::open(path).unwrap();
        let mut file = BufReader::new(file);
        let header: MdlHeader = bincode::deserialize_from(&mut file).unwrap();
        write!(&mut text, "{}", stem).unwrap();
        write!(&mut text, "{:#?}", header).unwrap();

        output_path.set_file_name(format!("{}.txt", stem));
        std::fs::write(&output_path, text).unwrap();
    }

    println!("Done!");
}
