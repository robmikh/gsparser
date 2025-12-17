use std::path::PathBuf;
use std::time::Duration;

use gsparser::mdl::MdlFile;
use gsparser::util::resolve_null_terminated_string;

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

    println!("Sequences:");
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
        println!(
            "  {} ({} fps) {:?}",
            animation_name, sequence.fps, sequence.linear_movement
        );
        let seconds_per_frame = 1.0 / sequence.fps;
        let frame_duration = Duration::from_secs_f32(seconds_per_frame);
        for event in events {
            let time = frame_duration * event.frame as u32;
            let options = event.options_string().unwrap_or("ERROR");
            println!(
                "    {} ({} ms) - {:?}   - {:?}",
                event.frame,
                time.as_millis(),
                event.event,
                options,
            );
        }
    }
}
