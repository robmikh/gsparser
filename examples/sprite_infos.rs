use std::path::PathBuf;

use gsparser::spr::SprFile;
use gsparser::sprite_info::SpriteInfoFile;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let game_root = args.get(0).unwrap();
    let sprite_name = args.get(1).unwrap();

    let sprite_path = {
        let mut path = PathBuf::from(game_root);
        path.push("sprites");
        path.push(sprite_name);
        path.set_extension("txt");
        path
    };

    let text = std::fs::read_to_string(sprite_path).unwrap();
    let file = SpriteInfoFile::parse(&text);
    println!("{:#?}", file);

    let mut output_path = {
        let mut path = PathBuf::from("testoutput");
        path.push(sprite_name);
        let _ = std::fs::create_dir_all(&path);
        path.push("something");
        path
    };
    println!("Sprites ({}):", file.infos.len());
    for (i, info) in file.infos.iter().enumerate() {
        let unique_name = format!("{}_{}", info.resolution, info.name);
        println!("  {} - {}", i, unique_name);

        // Load the spr file
        let spr_path = {
            let mut path = PathBuf::from(game_root);
            path.push("sprites");
            path.push(&info.file_path);
            path.set_extension("spr");
            path
        };
        let bytes = std::fs::read(spr_path).unwrap();
        let spr_file = SprFile::from_bytes(&bytes);

        let image = spr_file.decode_sprite(info);

        output_path.set_file_name(format!("{}.png", unique_name));
        image
            .save_with_format(&output_path, image::ImageFormat::Png)
            .unwrap();
    }
}
