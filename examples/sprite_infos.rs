use std::fmt::Write;
use std::{collections::HashMap, path::PathBuf};

use gsparser::mdl::{null_terminated_bytes_to_str, MdlFile};
use gsparser::spr::SprFile;
use gsparser::sprite_info::SpriteInfoFile;
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
        path.set_extension("txt");
        path
    };

    let text = std::fs::read_to_string(sprite_path).unwrap();
    let file = SpriteInfoFile::parse(&text);
    println!("{:#?}", file);
}
