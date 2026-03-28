use std::fmt::Write;
use std::path::PathBuf;
use std::time::Duration;

use gsparser::mdl::MdlFile;
use gsparser::steam::get_half_life_steam_install_path;
use gsparser::util::resolve_null_terminated_string;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let (game_root, output_path) = if args.len() == 1 {
        // Infer the game root via Steam
        let game_root =
            get_half_life_steam_install_path().expect("Failed to find Half-Life install location!");
        // Arg is the output path
        let output_path = PathBuf::from(args.get(0).unwrap());
        (game_root, output_path)
    } else if args.len() == 2 {
        let game_root = PathBuf::from(args.get(0).unwrap());
        let output_path = PathBuf::from(args.get(1).unwrap());
        (game_root, output_path)
    } else {
        panic!("Expected file output path!");
    };

    let model_directory = PathBuf::from(game_root).join("models");

    for entry in std::fs::read_dir(&model_directory)? {
        let entry = entry.unwrap();
        let entry_path = entry.path();
        if let Some(extension) = entry_path.extension() {
            if let Some(extension) = extension.to_str() {
                if extension == "mdl" {
                    let stem = {
                        if let Some(stem) = entry_path.file_stem() {
                            stem.to_str().unwrap().to_string()
                        } else {
                            continue;
                        }
                    };
                    println!("Processing {}.mdl...", stem);
                    if let Ok(file) = MdlFile::open(&entry_path) {
                        let mut output = String::new();

                        writeln!(&mut output, "Sequences:")?;
                        for (sequence, events) in file
                            .animation_sequences
                            .iter()
                            .zip(file.animation_sequence_events.iter())
                        {
                            let is_movement_zero = sequence.linear_movement[0] == 0.0
                                && sequence.linear_movement[1] == 0.0
                                && sequence.linear_movement[2] == 0.0;
                            if events.is_empty() || is_movement_zero {
                                continue;
                            }
                            let animation_name = resolve_null_terminated_string(&sequence.name);
                            writeln!(
                                &mut output,
                                "  {} ({} fps) {:?}",
                                animation_name, sequence.fps, sequence.linear_movement
                            )?;
                            let seconds_per_frame = 1.0 / sequence.fps;
                            let frame_duration = Duration::from_secs_f32(seconds_per_frame);
                            for event in events {
                                let time = frame_duration * event.frame as u32;
                                let options = event.options_string().unwrap_or("ERROR");
                                writeln!(
                                    &mut output,
                                    "    {} ({} ms) - {:?}   - {:?}",
                                    event.frame,
                                    time.as_millis(),
                                    event.event,
                                    options,
                                )?;
                            }
                        }

                        let output_file_path = output_path.join(format!("{}.txt", stem));
                        std::fs::write(output_file_path, output)?;
                    } else {
                        println!("  Skipped!");
                    }
                }
            }
        }
    }

    Ok(())
}
