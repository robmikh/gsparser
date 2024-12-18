use gsparser::{
    demo::{parse_entry_frames, DemoDirectory, DemoFrameData, DemoHeader, Parse},
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
    println!();

    // Dump the LOADING entry
    let loading_index = directory
        .entries
        .iter()
        .position(|x| {
            let description = null_terminated_bytes_to_str(&x.description).unwrap();
            description == "LOADING"
        })
        .expect("Couldn't find \"LOADING\" entry!");
    let loading_frames = &frames[loading_index];
    for frame in loading_frames {
        match &frame.data {
            DemoFrameData::NetMsg((frame_ty, data)) => {
                let sky_name = null_terminated_bytes_to_str(&data.prefix.info.move_vars.sky_name).unwrap();
                println!("{:#?}", data.prefix);
                println!("sky_name: {}", sky_name);
                println!();
            },
            DemoFrameData::DemoStart => todo!(),
            DemoFrameData::ConsoleCommand(console_command_data) => todo!(),
            DemoFrameData::ClientData(client_data_data) => todo!(),
            DemoFrameData::NextSection => todo!(),
            DemoFrameData::Event(event_data) => todo!(),
            DemoFrameData::WeaponAnim(weapon_anim_data) => todo!(),
            DemoFrameData::Sound(sound_data) => todo!(),
            DemoFrameData::DemoBuffer(demo_buffer_data) => todo!(),
        }
    }

    println!("Success!");
}
