use std::fmt::Write;
use std::path::PathBuf;

use gsparser::spr::SprFile;
use gsparser::wad3::WadArchive;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let wad_file = args.get(0).unwrap();

    let archive = WadArchive::open(wad_file);

    for file_info in &archive.files {
        if file_info.name.starts_with("{shot") || file_info.name.starts_with("{blood") {
            let image_data = archive.decode_mipmaped_image_as_hl_decal(file_info).image;
            image_data
                .save(format!("testoutput/decals/{}.png", &file_info.name))
                .unwrap();
        }
    }
}
