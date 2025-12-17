use std::fmt::Write;
use std::path::PathBuf;

use gsparser::spr::SprFile;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let game_root = args.get(0).unwrap();
    let sprite_name = args.get(1).unwrap();
    let sprite_folder = args.get(2);

    let sprite_path = {
        let mut path = PathBuf::from(game_root);
        path.push("sprites");
        if let Some(sprite_folder) = sprite_folder {
            path.push(sprite_folder);
        }
        path.push(sprite_name);
        path.set_extension("spr");
        path
    };

    let bytes = std::fs::read(sprite_path).unwrap();
    let file = SprFile::from_bytes(&bytes);
    println!("{:#?}", file.header);

    let mut output_path = {
        let mut path = PathBuf::from("testoutput");
        path.push(sprite_name);
        let _ = std::fs::create_dir_all(&path);
        path.push("something");
        path
    };
    println!("Frames ({}):", file.frames.len());
    for i in 0..file.frames.len() {
        println!("  {}", i);
        let mut text = String::new();
        write!(&mut text, "{:#?}", &file.frames[i].header).unwrap();
        for line in text.lines() {
            println!("    {}", line);
        }
        let image = file.decode_frame(i);
        output_path.set_file_name(format!("{}.png", i));
        image
            .save_with_format(&output_path, image::ImageFormat::Png)
            .unwrap();
    }
}
