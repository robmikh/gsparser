use std::path::PathBuf;

use gsparser::mdl::{MdlFile, null_terminated_bytes_to_str};

pub fn resolve_string_bytes<'a>(bytes: &'a [u8]) -> std::borrow::Cow<'a, str> {
    match null_terminated_bytes_to_str(bytes) {
        Ok(entities) => std::borrow::Cow::Borrowed(entities),
        Err(error) => {
            //println!("  WARNING: {:?}", error);
            let start = error.str_error.valid_up_to();
            let _end = start + error.str_error.error_len().unwrap_or(1);
            //println!("           error bytes: {:?}", &bytes[start..end]);
            String::from_utf8_lossy(&bytes[..error.end])
        }
    }
}

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
