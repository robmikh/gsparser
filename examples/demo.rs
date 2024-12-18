use gsparser::{
    demo::{parse_entry_frames, DemoDirectory, DemoHeader, Parse},
    mdl::null_terminated_bytes_to_str,
};

fn main() {
    let demo_path = "testdata/testdemo.dem";
    let demo_bytes = std::fs::read(demo_path).unwrap();
    let bytes_len = demo_bytes.len();
    let mut reader = std::io::Cursor::new(demo_bytes);

    let header = DemoHeader::parse(&mut reader).unwrap();
    println!("{:?}", header);
    assert_eq!(&header.magic, b"HLDEMO\0\0");

    let map_name = null_terminated_bytes_to_str(&header.map_name).unwrap();
    let game_directory = null_terminated_bytes_to_str(&header.game_directory).unwrap();

    println!("map_name: {}", map_name);
    println!("game_directory: {}", game_directory);

    println!("len: {}", bytes_len);
    println!("offset: {}", header.directory_offset);

    reader.set_position(header.directory_offset as u64);

    let directory = DemoDirectory::parse(&mut reader).unwrap();
    println!("{} entries found", directory.len);

    for entry in &directory.entries {
        let description = null_terminated_bytes_to_str(&entry.description).unwrap();
        println!("  {}", description);
    }

    println!("Dumping frames...");
    let frames = parse_entry_frames(&mut reader, &directory.entries).unwrap();
    for (entry, frames) in directory.entries.iter().zip(&frames) {
        let description = null_terminated_bytes_to_str(&entry.description).unwrap();
        println!("  {}", description);
        for frame in frames {
            println!("    {:?}", frame.header.frame_ty);
            //println!("      {:?}", frame.data);
        }
    }

    println!("Success!");
}
