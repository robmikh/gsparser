fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let sav_path = args.get(0).unwrap();

    let bytes = std::fs::read(sav_path)?;

    // Sanity test
    let sanity_offset = 0x2D30;
    let mut sanity_bytes = [0u8; 4];
    sanity_bytes.copy_from_slice(&bytes[sanity_offset..sanity_offset+4]);
    println!("Sanity bytes: {:X?}", &sanity_bytes);

    // Look for: XXBD01
    // TODO: Is there a fixed or specified place to start?
    let mut current = 0;
    let window_len = 4;
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

            println!("  {:6X} {:04} {}", current, number, class_name);
            current = class_name_end + 1;
        } else {
            current += 1;
        }
    }



    Ok(())
}