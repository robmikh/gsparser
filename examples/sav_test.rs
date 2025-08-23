use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let sav_path = args.get(0).unwrap();

    let sav_path = PathBuf::from(sav_path);
    let sav_paths = 
    if sav_path.is_dir() {
        // Find all the sav files
        let mut sav_paths = Vec::new();
        for entry in std::fs::read_dir(sav_path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_file() {
                let path = entry.path();
                if let Some(extension) = path.extension() {
                    if let Some(extension) = extension.to_str() {
                        if extension == "sav" {
                            sav_paths.push(path);
                        }
                    }
                }
            }
        }
        sav_paths
    } else {
        vec![sav_path]
    };
    
    for sav_path in sav_paths {
        println!("Processing: {}", sav_path.display());
        process_path(sav_path)?;
        println!();
    }

    Ok(())
}

fn find_next_null(bytes: &[u8], start: usize) -> Option<usize> {
    let mut end = start;
    while end < bytes.len() {
        if bytes[end] == 0 {
            return Some(end);
        }
        end += 1;
    }
    None
}

fn process_path<P: AsRef<Path>>(sav_path: P) -> Result<(), Box<dyn std::error::Error>> {
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
            let class_name_end = find_next_null(&bytes, class_name_start).unwrap();
            let class_name_bytes = &bytes[class_name_start..class_name_end];
            let class_name_result = std::str::from_utf8(class_name_bytes);
            if class_name_result.is_err() {
                //println!("WARNING: Assuming false positive at {:X} due to invalid utf8 class name.", current);
                current += 1;
                continue;
            }
            let class_name = class_name_result?;

            if class_name.len() + 1 != number {
                //println!("WARNING: Assuming false positive at {:X} due to wrong class name length. ", current);
                current += 1;
                continue;
            }

            offsets_and_ends.push((current, class_name_end));
            if first_offset_and_class_name.is_none() {
                first_offset_and_class_name = Some((current, class_name));
            }

            //println!("  {:6X} {:04} {}", current, number, class_name);
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

        //println!("  {:6X} {:8} {:8} {:8}  {}", offset, offset_distance, end_distance, end_to_next_offset, class_name);
    }

    println!("There are {} pairs.", offsets_and_ends.len());
    println!("There are {} pairs that have a worldspawn class name", num_world_spawn);
    // Doesn't work with all my saves...
    //assert!(offsets_and_ends.len() % num_world_spawn == 0, "Number of pairs {} are not divisible by {}", offsets_and_ends.len(), num_world_spawn);

    // Find map name
    let map_name_start = 0x106E;
    let map_name_end = find_next_null(&bytes, map_name_start).unwrap();
    let map_name_bytes = &bytes[map_name_start..map_name_end];
    let map_name = std::str::from_utf8(map_name_bytes)?;
    println!("Map name: {}", map_name);

    Ok(())
}