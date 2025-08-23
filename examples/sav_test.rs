fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let sav_path = args.get(0).unwrap();

    process_path(sav_path)?;

    Ok(())
}

fn process_path(sav_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(sav_path)?;

    // Look for: XXBD01
    // TODO: Is there a fixed or specified place to start?
    let mut current = 0;
    let window_len = 4;
    let mut offsets_and_ends = Vec::new();
    let mut first_offset_and_class_name = None;
    while current + window_len < bytes.len() {
        let current_bytes = &bytes[current..current + window_len];

        if current_bytes[2] == 0xBD && current_bytes[3] == 0x01 {
            let number = u16::from_le_bytes(current_bytes[0..2].try_into()?) as usize;

            let class_name_start = current + window_len;
            let mut class_name_end = class_name_start;
            while class_name_end < bytes.len() {
                if bytes[class_name_end] == 0 {
                    break;
                }
                class_name_end += 1;
            }
            let class_name_bytes = &bytes[class_name_start..class_name_end];
            let class_name = std::str::from_utf8(class_name_bytes)?;

            if class_name.len() + 1 != number {
                println!("WARNING: Assuming false positive at {:X}. ", current);
                current += 1;
                continue;
            }

            offsets_and_ends.push((current, class_name_end));
            if first_offset_and_class_name.is_none() {
                first_offset_and_class_name = Some((current, class_name));
            }

            println!("  {:6X} {:04} {}", current, number, class_name);
            current = class_name_end + 1;
        } else {
            current += 1;
        }
    }

    // The first entry should be the worldspawn entity
    assert_eq!(first_offset_and_class_name.map(|(_, class_name)| class_name), Some("worldspawn"));
    
    let mut num_world_spawn = 0;
    for pairs in offsets_and_ends.windows(2) {
        let offset = pairs[0].0;
        let end = pairs[0].1;
        let offset_2 = pairs[1].0;
        let end_2 = pairs[1].1;

        let offset_distance = offset_2 - offset;
        let end_distance = end_2 - end;
        let end_to_next_offset = offset_2 - end;

        let class_name_start = offset + window_len;
        let class_name_bytes = &bytes[class_name_start..end];
        let class_name = std::str::from_utf8(class_name_bytes)?;

        if class_name == "worldspawn" {
            num_world_spawn += 1;
        }

        println!("  {:6X} {:8} {:8} {:8}  {}", offset, offset_distance, end_distance, end_to_next_offset, class_name);
    }

    println!("There are {} pairs.", offsets_and_ends.len());
    println!("There are {} pairs that have a worldspawn class name", num_world_spawn);
    // Doesn't work with all my saves...
    //assert!(offsets_and_ends.len() % num_world_spawn == 0, "Number of pairs {} are not divisible by {}", offsets_and_ends.len(), num_world_spawn);

    Ok(())
}