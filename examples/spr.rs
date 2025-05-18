use std::fmt::Write;
use std::{collections::HashMap, path::PathBuf};

use gsparser::mdl::{MdlFile, null_terminated_bytes_to_str};
use gsparser::spr::SprFile;
use id_tree::InsertBehavior::AsRoot;
use id_tree::InsertBehavior::UnderNode;
use id_tree::TreeBuilder;

pub fn resolve_string_bytes<'a>(bytes: &'a [u8]) -> std::borrow::Cow<'a, str> {
    match null_terminated_bytes_to_str(bytes) {
        Ok(entities) => std::borrow::Cow::Borrowed(entities),
        Err(error) => {
            //println!("  WARNING: {:?}", error);
            let start = error.str_error.valid_up_to();
            let end = start + error.str_error.error_len().unwrap_or(1);
            //println!("           error bytes: {:?}", &bytes[start..end]);
            String::from_utf8_lossy(&bytes[..error.end])
        }
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let game_root = args.get(0).unwrap();
    let sprite_name = args.get(1).unwrap();

    let sprite_path = {
        let mut path = PathBuf::from(game_root);
        path.push("sprites");
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
