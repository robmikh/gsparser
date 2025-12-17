use std::path::PathBuf;

use gsparser::mdl::MdlFile;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let game_root = args.get(0).unwrap();
    let model_name = args.get(1).unwrap();

    let model_path = {
        let mut path = PathBuf::from(game_root);
        path.push("models");
        path.push(model_name);
        path.set_extension("mdl");
        path
    };

    let file = MdlFile::open(model_path).unwrap();
    println!("{:#?}", file.header);

    println!("Hit boxes ({}):", file.hit_boxes.len());
    for (i, hit_box) in file.hit_boxes.iter().enumerate() {
        println!("  {} - {:?}", i, hit_box);
    }
}
