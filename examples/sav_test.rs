fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let sav_path = args.get(0).unwrap();

    process_path(sav_path)?;

    Ok(())
}

fn process_path(sav_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(sav_path)?;

    // Sanity test
    let sanity_offset = 0x2D30;
    let mut sanity_bytes = [0u8; 4];
    sanity_bytes.copy_from_slice(&bytes[sanity_offset..sanity_offset+4]);
    println!("Sanity bytes: {:X?}", &sanity_bytes);
    assert_eq!(sanity_bytes, [0xB, 0x0, 0xBD, 0x1]);

    // Look for: XXBD01
    // TODO: Is there a fixed or specified place to start?
    let mut current = 0;
    let window_len = 4;
    let mut offsets_and_ends = Vec::new();
    while current + window_len < bytes.len() {
        let current_bytes = &bytes[current..current + window_len];

        if current_bytes[2] == 0xBD && current_bytes[3] == 0x01 {
            let number = u16::from_le_bytes(current_bytes[0..2].try_into()?);

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
            assert_eq!(class_name.len() + 1, number as usize, "\"{}\" at offset {} ({:X}) should be {} bytes long, is {}.", class_name, current, current, number, class_name.len() + 1);
            offsets_and_ends.push((current, class_name_end));

            println!("  {:6X} {:04} {}", current, number, class_name);
            current = class_name_end + 1;
        } else {
            current += 1;
        }
    }
    
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

        println!("  {:6X} {:8} {:8} {:8}  {}", offset, offset_distance, end_distance, end_to_next_offset, class_name);
    }

    println!("There are {} pairs.", offsets_and_ends.len());
    Ok(())
}