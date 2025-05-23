use gsparser::{
    demo::{DemoDirectory, DemoFrameData, DemoHeader, Parse, parse_entry_frames},
    mdl::null_terminated_bytes_to_str,
};

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
    let demo_path = if args.is_empty() {
        "testdata/testdemo.dem"
    } else {
        &args[0]
    };
    let demo_bytes = std::fs::read(demo_path).unwrap();
    let bytes_len = demo_bytes.len();
    let mut reader = std::io::Cursor::new(demo_bytes);

    let header = DemoHeader::parse(&mut reader).unwrap();
    println!("{:?}", header);
    assert_eq!(&header.magic, b"HLDEMO\0\0");

    assert_eq!(header.demo_protocol, 5, "Unsupported protocol version!");

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
        let mut last_frame = 0;
        for (i, frame) in frames.iter().enumerate() {
            if last_frame != frame.header.frame {
                println!();
            }
            match &frame.data {
                DemoFrameData::NetMsg((frame_ty, data)) => {
                    let sky_name =
                        null_terminated_bytes_to_str(&data.prefix.info.move_vars.sky_name).unwrap();
                    let side_move = data.prefix.info.user_cmd.sidemove;
                    let forward_move = data.prefix.info.user_cmd.forwardmove;
                    let up_move = data.prefix.info.user_cmd.upmove;
                    let position = data.prefix.info.ref_params.view_org;
                    //println!("{:#?}", data.prefix);
                    //println!("sky_name: {}", sky_name);
                    println!(
                        "    {} - NetMsg - (forward, side, up) {}, {}, {}  position: {:?}",
                        frame.header.frame, forward_move, side_move, up_move, position
                    );
                }
                DemoFrameData::DemoStart => println!("    {} - Demo Start", frame.header.frame),
                DemoFrameData::ConsoleCommand(console_command_data) => {
                    //println!("Command: {:?}", console_command_data);
                    let command = resolve_string_bytes(&console_command_data.data);
                    println!("    {} - Console command: {}", frame.header.frame, command);
                }
                DemoFrameData::ClientData(client_data_data) => {
                    let position = client_data_data.origin;
                    println!("    {} - Client Data - {:?}", frame.header.frame, position);
                }
                DemoFrameData::NextSection => todo!(),
                DemoFrameData::Event(event_data) => todo!(),
                DemoFrameData::WeaponAnim(weapon_anim_data) => todo!(),
                DemoFrameData::Sound(sound_data) => {
                    println!("    {} - Sound Data", frame.header.frame)
                }
                DemoFrameData::DemoBuffer(demo_buffer_data) => {
                    println!("    {} - Demo Buffer", frame.header.frame);
                    println!("      {:?}", demo_buffer_data.data.data);
                }
            }
            last_frame = frame.header.frame;
        }
    }
    println!();

    println!("Success!");
}
