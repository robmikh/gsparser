use std::{borrow::Cow, fmt::Write, io::Read, path::{Path, PathBuf}};

use gsparser::{bsp::{BspEntity, BspReader}, mdl::null_terminated_bytes_to_str};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let sav_path = args.get(0).unwrap();
    let game_root = args.get(1).unwrap();
    let output_path = args.get(2).unwrap();

    let sav_path = PathBuf::from(sav_path);
    let game_root = PathBuf::from(game_root);
    let output_path = PathBuf::from(output_path);

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
        let data = process_path(&sav_path)?;
        println!("  num worldspawn: {}", data.num_world_spawns);

        let map_path = {
            let mut path = game_root.clone();
            path.push("maps");
            path.push(&data.map_name);
            path.set_extension("bsp");
            path
        };
        println!("  Loading map from {}", map_path.display());
        let bsp_data = std::fs::read(map_path)?;
        let reader = BspReader::read(bsp_data);
        let entity_string = resolve_map_entity_string(&reader);
        let entities = BspEntity::parse_entities(&entity_string);
        let num_entities = entities.len();

        println!("  num_pairs: {}    num_entities: {}    remainder: {}    divided: {}", data.num_entries, num_entities, data.num_entries % num_entities, data.num_entries as f64 / num_entities as f64);
        //assert!(data.num_pairs % num_entities == 0, "num_pairs: {}    num_entities: {}    remainder: {}    divided: {}", data.num_entries, num_entities, data.num_entries % num_entities, data.num_entries as f64 / num_entities as f64);
    
        assert!(data.num_world_spawns > 1);
        for window in data.world_spawn_indices.windows(2) {
            let first_index = window[0];
            let second_index = window[1];
            let between_world_spawns = second_index - first_index;
            println!("  Entries between world spawns {} and {}: {}", first_index, second_index, between_world_spawns);
        }

        let sav_output_path = {
            let mut path = output_path.clone();
            path.push(sav_path.file_name().unwrap().to_str().unwrap());
            path.set_extension("txt");
            path
        };
        std::fs::write(sav_output_path, &data.output)?;

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

fn find_next_non_null(bytes: &[u8], start: usize) -> Option<usize> {
    let mut end = start;
    while end < bytes.len() {
        if bytes[end] != 0 {
            return Some(end);
        }
        end += 1;
    }
    None
}

struct SavData {
    map_name: String,
    num_entries: usize,
    num_world_spawns: usize,
    entries: Vec<(usize, String)>,
    world_spawn_indices: Vec<usize>,
    output: String,
}

fn read_u32_le<R: Read>(mut reader: R) -> std::io::Result<u32> {
    let mut value = [0u8; 4];
    reader.read_exact(&mut value)?;
    Ok(u32::from_le_bytes(value))
}

fn read_u16_le<R: Read>(mut reader: R) -> std::io::Result<u16> {
    let mut value = [0u8; 2];
    reader.read_exact(&mut value)?;
    Ok(u16::from_le_bytes(value))
}

fn process_path<P: AsRef<Path>>(sav_path: P) -> Result<SavData, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(sav_path)?;

    let mut output = String::new();

    let mut reader = std::io::Cursor::new(&bytes);
    // Header
    let magic = {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        magic
    };
    assert_eq!(&magic, b"JSAV");
    let version = read_u32_le(&mut reader)?;
    assert_eq!(version, 0x71);
    let door_info_len = read_u32_le(&mut reader)?;
    let token_count = read_u32_le(&mut reader)?;
    let tokens_size = read_u32_le(&mut reader)?;
    writeln!(&mut output, "Header:")?;
    writeln!(&mut output, "  magic: {:X?}", magic)?;
    writeln!(&mut output, "  version: 0x{:X}", version)?;
    writeln!(&mut output, "  door_info_len: {} (0x{:X})", door_info_len, door_info_len)?;
    writeln!(&mut output, "  token_count: {} (0x{:X})", token_count, token_count)?;
    writeln!(&mut output, "  tokens_size: {} (0x{:X})", tokens_size, tokens_size)?;
    writeln!(&mut output, "")?;

    // Read tokens data
    let (num, tokens) = read_token_table(&mut reader, tokens_size as usize)?;
    writeln!(&mut output, "Tokens ({}):", tokens.len())?;
    for (offset, token) in &tokens {
        writeln!(&mut output, "  ({:4})  \"{}\"", offset, token)?;
    }
    let num_message = if num == token_count { "matches token_count!" } else { "NO MATCH" };
    writeln!(&mut output, "Num: {} ({})", num, num_message)?;
    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;
    assert_eq!(num, token_count);

    // Read "door info"
    let door_info_start = offset;
    let door_info_bytes = {
        let mut door_info_bytes = vec![0u8; door_info_len as usize];
        reader.read_exact(&mut door_info_bytes)?;
        door_info_bytes
    };
    let _ = process_door_infos(&door_info_bytes, 0x94, &mut output)?;
    let mut door_info_reader = std::io::Cursor::new(&door_info_bytes);
    let (_, game_header_struct) = read_struct(&mut door_info_reader, Some("GameHeader"), &tokens, &mut output)?;
    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;
    let offset = door_info_reader.position();
    writeln!(&mut output, "Current Door Info Offset (Relative): {} (0x{:X})", offset, offset)?;
    writeln!(&mut output, "Current Door Info Offset: {} (0x{:X})", door_info_start + offset, door_info_start + offset)?;

    let (_, global_struct) = read_struct(&mut door_info_reader, Some("GLOBAL"), &tokens, &mut output)?;
    let offset = door_info_reader.position();
    writeln!(&mut output, "Current Door Info Offset (Relative): {} (0x{:X})", offset, offset)?;
    writeln!(&mut output, "Current Door Info Offset: {} (0x{:X})", door_info_start + offset, door_info_start + offset)?;

    // We should have a 'm_listCount' with the number of door infos
    let list_count = read_u32_field(&global_struct, "m_listCount").unwrap();
    let mut door_infos = Vec::with_capacity(list_count as usize);
    for _ in 0..list_count {
        let (_, gent_struct) = read_struct(&mut door_info_reader, Some("GENT"), &tokens, &mut output)?;
        
        let name = read_str_field(&gent_struct, "name")?;
        let level_name = read_str_field(&gent_struct, "levelName")?;

        let state_bytes = get_field(&gent_struct, "state").map(|bytes| bytes.clone());

        door_infos.push((name.to_owned(), level_name.to_owned(), state_bytes.clone()));
    }
    writeln!(&mut output, "Door infos ({}):", list_count)?;
    for (name, level_name, state_bytes) in &door_infos {
        writeln!(&mut output, "  {:24} ({:10}) ({:02X?})", name, level_name, state_bytes)?;
    }

    //let num_state_files = read_u16_le(&mut reader)?;
    let offset = door_info_reader.position();
    writeln!(&mut output, "Current Door Info Offset (Relative): {} (0x{:X})", offset, offset)?;
    writeln!(&mut output, "Current Door Info Offset: {} (0x{:X})", door_info_start + offset, door_info_start + offset)?;

    let hl1_header_start = reader.position();
    let (hl1_name, hl1_header, hl1_block) = read_hl_block(&mut reader)?;
    let hl1_block_start = hl1_header_start + hl1_header.len() as u64 + 4;
    writeln!(&mut output, "HL1 Name: \"{}\"", hl1_name)?;

    let (hl2_name, hl2_header, hl2_block) = read_hl_block(&mut reader)?;
    writeln!(&mut output, "HL2 Name: \"{}\"", hl2_name)?;

    let (hl3_name, hl3_header, hl3_block) = read_hl_block(&mut reader)?;
    writeln!(&mut output, "HL3 Name: \"{}\"", hl3_name)?;

    // How many are there?
    let mut num_blocks = 3;
    while (reader.position() as usize) < bytes.len() {
        let (hl1_name, hl1_header, hl1_block) = read_hl_block(&mut reader)?;
        writeln!(&mut output, "HLX Name: \"{}\"", hl1_name)?;
        num_blocks += 1;
    }
    writeln!(&mut output, "Num HL blocks: {} (0x{:X})", num_blocks, num_blocks)?;
    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;
    assert_eq!(offset as usize, bytes.len());

    // Poke at the first HL1 block
    let mut hl1_block_reader = std::io::Cursor::new(&hl1_block);
    let magic = {
        let mut magic = [0u8; 4];
        hl1_block_reader.read_exact(&mut magic)?;
        magic
    };
    assert_eq!(&magic, b"VALV");
    let version = read_u32_le(&mut hl1_block_reader)?;
    assert_eq!(version, 0x71);
    let unknown_1 = read_u32_le(&mut hl1_block_reader)?;
    writeln!(&mut output, "  unknown_1: {} (0x{:X})", unknown_1, unknown_1)?;
    let expected_num_etables = read_u32_le(&mut hl1_block_reader)?;
    writeln!(&mut output, "  expected_num_etables: {} (0x{:X})", expected_num_etables, expected_num_etables)?;
    let token_count = read_u32_le(&mut hl1_block_reader)?;
    writeln!(&mut output, "  token_count: {} (0x{:X})", token_count, token_count)?;
    let token_table_len = read_u32_le(&mut hl1_block_reader)?;
    writeln!(&mut output, "  token_table_len: {} (0x{:X})", token_table_len, token_table_len)?;
    let (num, tokens) = read_token_table(&mut hl1_block_reader, token_table_len as usize)?;
    writeln!(&mut output, "Tokens ({}):", tokens.len())?;
    for (offset, token) in &tokens {
        writeln!(&mut output, "  ({:4})  \"{}\"", offset, token)?;
    }
    let num_message = if num == token_count { "matches token_count!" } else { "NO MATCH" };
    writeln!(&mut output, "Num: {} ({})", num, num_message)?;
    assert_eq!(num, token_count);

    let offset = hl1_block_reader.position();
    writeln!(&mut output, "Current HL1 Block Offset (Relative): {} (0x{:X})", offset, offset)?;
    writeln!(&mut output, "Current HL1 Block Offset: {} (0x{:X})", hl1_block_start + offset, hl1_block_start + offset)?;

    let mut num_etables = 0;
    for _ in 0..expected_num_etables {
        let etable_struct = read_struct(&mut hl1_block_reader, Some("ETABLE"), &tokens, &mut output)?;
        num_etables += 1;
    }
    writeln!(&mut output, "num_etables: {} ({})", num_etables, num_etables)?;
    assert_eq!(num_etables, expected_num_etables);

    let (_, save_header) = read_struct(&mut hl1_block_reader, Some("Save Header"), &tokens, &mut output)?;
    let connection_count = read_u32_field(&save_header, "connectionCount").unwrap();
    for _ in 0..connection_count {
        let (_, adjacency_data) = read_struct(&mut hl1_block_reader, Some("ADJACENCY"), &tokens, &mut output)?;
    }
    
    // Read "LIGHTSTYLE" structs
    let light_style_count = read_u32_field(&save_header, "lightStyleCount").unwrap();
    for _ in 0..light_style_count {
        let (_, light_style) = read_struct(&mut hl1_block_reader, Some("LIGHTSTYLE"), &tokens, &mut output)?;
    }

    // Read "ENTVARS" structs
    let entity_count = read_u32_field(&save_header, "entityCount").unwrap();
    //println!("entity_count: {}", entity_count);
    //println!("num_etables: {}", num_etables);
    let mut current_entity: Option<Vec<(&str, Vec<(&str, Vec<u8>)>)>> = None;
    let mut entities = Vec::with_capacity(entity_count as usize);
    while entities.len() < entity_count as usize {
        //let offset = hl1_block_reader.position() + hl1_block_start;
        //println!("  offset: 0x{:X}", offset);
        let (ty, entity_vars) = match read_struct(&mut hl1_block_reader, None, &tokens, &mut output) {
            Ok(result) => result,
            Err(error) => {
                writeln!(&mut output, "ERROR: {}", error)?;
                break;
            }
        };
        if ty == "ENTVARS" {
            if let Some(current_entity) = current_entity.take() {
                entities.push(current_entity);
            }
        }
        if let Some(current_entity) = current_entity.as_mut() {
            current_entity.push((ty, entity_vars));
        } else {
            let mut entity_fragments = Vec::new();
            entity_fragments.push((ty, entity_vars));
            current_entity = Some(entity_fragments);
        }
    }
    writeln!(&mut output, "Entities:")?;
    for entity in entities {
        // The first should be ENTVARS which should have a class name
        let class_name = read_str_field(&entity[0].1, "classname")?;
        writeln!(&mut output, "  {}", class_name)?;
        for fragment in &entity {
        writeln!(&mut output, "    {}", fragment.0)?;
            record_fields(&fragment.1, "      ", &mut output)?;
        }
    }


    let offset = hl1_block_reader.position();
    writeln!(&mut output, "Current HL1 Block Offset (Relative): {} (0x{:X})", offset, offset)?;
    writeln!(&mut output, "Current HL1 Block Offset: {} (0x{:X})", hl1_block_start + offset, hl1_block_start + offset)?;


    writeln!(&mut output, "")?;

    // Look for: XXBD01
    // TODO: Is there a fixed or specified place to start?
    let mut current = 0;
    let window_len = 4;
    let mut offsets_and_ends = Vec::new();
    let mut first_offset_and_class_name = None;
    let mut entries = Vec::new();
    let mut world_spawn_indices = Vec::new();
    writeln!(&mut output, "XXBD01 Matches:")?;
    while current + window_len < bytes.len() {
        let current_bytes = &bytes[current..current + window_len];

        if current_bytes[2] == 0xBD && current_bytes[3] == 0x01 {
            let number = u16::from_le_bytes(current_bytes[0..2].try_into()?) as usize;

            let class_name_start = current + window_len;
            let class_name_end = find_next_null(&bytes, class_name_start).unwrap();
            let class_name_bytes = &bytes[class_name_start..class_name_end];
            let class_name_result = std::str::from_utf8(class_name_bytes);
            if class_name_result.is_err() {
                writeln!(&mut output, "WARNING: Assuming false positive at {:X} due to invalid utf8 class name.", current)?;
                current += 1;
                continue;
            }
            let class_name = class_name_result?;

            if class_name.len() + 1 != number {
                writeln!(&mut output, "WARNING: Assuming false positive at {:X} due to wrong class name length. ", current)?;
                current += 1;
                continue;
            }

            offsets_and_ends.push((current, class_name_end));
            entries.push((current, class_name.to_owned()));
            if first_offset_and_class_name.is_none() {
                first_offset_and_class_name = Some((current, class_name));
            }
            if class_name == "worldspawn" {
                world_spawn_indices.push(entries.len() - 1);
            }

            writeln!(&mut output, "  {:6X} {:04} {}", current, number, class_name)?;
            current = class_name_end + 1;
        } else {
            current += 1;
        }
    }

    // The first entry should be the worldspawn entity
    assert_eq!(first_offset_and_class_name.map(|(_, class_name)| class_name), Some("worldspawn"));
    
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

        writeln!(&mut output, "  {:6X} {:8} {:8} {:8}  {}", offset, offset_distance, end_distance, end_to_next_offset, class_name)?;
    }

    writeln!(&mut output, "There are {} pairs.", offsets_and_ends.len())?;
    writeln!(&mut output, "There are {} pairs that have a worldspawn class name", world_spawn_indices.len())?;
    // Doesn't work with all my saves...
    //assert!(offsets_and_ends.len() % num_world_spawn == 0, "Number of pairs {} are not divisible by {}", offsets_and_ends.len(), num_world_spawn);

    // Find map name
    let map_name_start = 0x106E;
    let map_name_end = find_next_null(&bytes, map_name_start).unwrap();
    let map_name_bytes = &bytes[map_name_start..map_name_end];
    let map_name = std::str::from_utf8(map_name_bytes)?;
    writeln!(&mut output, "Map name: {}", map_name)?;

    let num_entries = entries.len();
    let num_world_spawns = world_spawn_indices.len();

    // Read door infos (?)
    let door_info_count_offset = 0x10EE;
    let door_info_offset = process_door_infos(&bytes, door_info_count_offset, &mut output)?;

    // The next chunk of data contains the map name, the substrings "HL1" and
    // "sav", and something that ends in "VALVq".
    let valvq_block_len = 272;
    let valvq_block_start = door_info_offset;
    let valvq_block_bytes = &bytes[valvq_block_start..valvq_block_start+valvq_block_len];
    assert_eq!(valvq_block_len, valvq_block_bytes.len());
    assert_eq!(valvq_block_bytes[0], 0);
    assert_eq!(valvq_block_bytes[1], 0);
    let suffix = &valvq_block_bytes[valvq_block_len-5..];
    assert_eq!(suffix, b"VALVq");

    // Next are a bunch of string inconsistently padded with 0s before we hit
    // the first entity class name ("worldspawn")
    let strings_offset = valvq_block_start + valvq_block_len + 1 + 16;
    let mut current_strings_offset = strings_offset;
    let mut num_strings = 0;
    let first_worldspawn = first_offset_and_class_name.map(|(offset, _)| offset).unwrap();
    writeln!(&mut output, "Strings:")?;
    while current_strings_offset < first_worldspawn {
        current_strings_offset = find_next_non_null(&bytes, current_strings_offset).unwrap();
        let string_end = find_next_null(&bytes, current_strings_offset).unwrap();
        let string_bytes = &bytes[current_strings_offset..string_end];
        //println!("{:X}  {:02X?}", current_strings_offset, string_bytes);
        let string = std::str::from_utf8(string_bytes)?;
        writeln!(&mut output, "  {}", string)?;
        current_strings_offset = string_end + 1;
        num_strings += 1;

        if string == "noise1" {
            break;
        }
    }
    writeln!(&mut output, "Found {} strings(s)", num_strings)?;

    Ok(SavData {
        map_name: map_name.to_owned(),
        num_entries,
        num_world_spawns,
        entries,
        world_spawn_indices,
        output,
    })
}

fn resolve_map_entity_string<'a>(reader: &'a BspReader) -> Cow<'a, str> {
    let entities_bytes = reader.read_entities();
    match null_terminated_bytes_to_str(entities_bytes) {
        Ok(entities) => Cow::Borrowed(entities),
        Err(error) => {
            println!("  WARNING: {:?}", error);
            let start = error.str_error.valid_up_to();
            let end = start + error.str_error.error_len().unwrap_or(1);
            println!("           error bytes: {:?}", &entities_bytes[start..end]);
            String::from_utf8_lossy(&entities_bytes[..error.end])
        }
    }
}

fn process_door_infos(bytes: &[u8], start: usize, output: &mut String) -> Result<usize, Box<dyn std::error::Error>> {
    let num_door_infos = bytes[start] as usize;
    let mut door_info_offset = start + 1;
    let mut saw_non_default_size_clue = false;
    writeln!(output, "Door infos:")?;
    for _ in 0..num_door_infos {
        // Check the first 15 bytes
        let prefix = &bytes[door_info_offset..door_info_offset+15];
        let expected = [0x00, 0x00, 0x00, 0x04, 0x00, 0xF1, 0x07, 0x03, 0x00, 0x00, 0x00, 0x40, 0x00, 0x1A, 0x0F];
        // The first 7 seem to be constant
        assert_eq!(&prefix[..7], &expected[..7]);
        // The rest of the bytes after the size clude also seem to be constant
        assert_eq!(&prefix[8..], &expected[8..]);
        let size_clue = prefix[7];
        if size_clue != 0x3 {
            saw_non_default_size_clue = true;
        }
        let size = match size_clue {
            0x3 => 120,
            0x2 => 112,
            _ => panic!("Unknown size clue \"{:02X}\"!", size_clue),
        };

        let data = &bytes[door_info_offset..door_info_offset+size];
        door_info_offset += size;

        let target_name_start = 15;
        let target_name_end = find_next_null(data, target_name_start).unwrap();
        let target_name_bytes = &data[target_name_start..target_name_end];
        let target_name = std::str::from_utf8(target_name_bytes)?;
        assert!(!target_name.is_empty());

        let entity_map_name_start = 83;
        let entity_map_name_end = find_next_null(data, entity_map_name_start).unwrap();
        let entity_map_name_bytes = &data[entity_map_name_start..entity_map_name_end];
        let entity_map_name = std::str::from_utf8(entity_map_name_bytes)?;
        assert!(!entity_map_name.is_empty());

        writeln!(output, "  {}  ({})", target_name, entity_map_name)?;
    }
    if saw_non_default_size_clue {
        writeln!(output, "Saw non-default size clue!")?;
    }
    Ok(door_info_offset)
}

fn read_struct<'a, R: Read>(mut reader: R, expected_name: Option<&str>, tokens: &'a [(u32, String)], output: &mut String) -> Result<(&'a str, Vec<(&'a str, Vec<u8>)>), Box<dyn std::error::Error>> {
    let always_4 = read_u16_le(&mut reader)?;
    assert_eq!(always_4, 4);
    let token_offset = read_u16_le(&mut reader)?;
    let token = tokens.iter().find(|(offset, _)|  *offset == token_offset as u32).map(|(_, token)| token.as_str()).unwrap();
    //assert_eq!(token, expected_name);
    if let Some(expected_name) = expected_name {
        if token != expected_name {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Expected \"{}\", found \"{}\"!", expected_name, token))));
        }
    }
    writeln!(output, "\"{}\":", token)?;
    let fields_saved = read_u16_le(&mut reader)?;
    writeln!(output, "  Fields: {} (0x{:X})", fields_saved, fields_saved)?;
    // Not what this short is for
    let unknown = read_u16_le(&mut reader)?;
    assert_eq!(unknown, 0);

    // Read each field
    let mut fields = Vec::with_capacity(fields_saved as usize);
    for _ in 0..fields_saved {
        let payload_size = read_u16_le(&mut reader)?;
        let token_offset = read_u16_le(&mut reader)?;
        let token = tokens.iter().find(|(offset, _)|  *offset == token_offset as u32).map(|(_, token)| token.as_str()).unwrap();

        let mut payload = vec![0u8; payload_size as usize];
        reader.read_exact(&mut payload)?;
        fields.push((token, payload));
    }
    for (field_name, payload) in &fields {
        writeln!(output, "    \"{}\" {:02X?}", field_name, payload)?;
    }

    Ok((token, fields))
}

fn read_hl_block<R: Read>(mut reader: R) -> Result<(String, Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    let hl1_header_len = 260;
    let hl1_header = {
        let mut hl1_header = vec![0u8; hl1_header_len as usize];
        reader.read_exact(&mut hl1_header)?;
        hl1_header
    };
    let hl1_name_start = 0;
    let hl1_name_end = find_next_null(&hl1_header, hl1_name_start).unwrap_or(hl1_header.len());
    let hl1_name = str::from_utf8(&hl1_header[hl1_name_start..hl1_name_end])?;

    let hl1_block_len = read_u32_le(&mut reader)?;
    let hl1_block = {
        let mut hl1_block = vec![0u8; hl1_block_len as usize];
        reader.read_exact(&mut hl1_block)?;
        hl1_block
    };

    Ok((hl1_name.to_owned(), hl1_header, hl1_block))
}

fn read_token_table<R: Read>(mut reader: R, tokens_size: usize) -> Result<(u32, Vec<(u32, String)>), Box<dyn std::error::Error>> {
    let tokens_data = {
        let mut tokens_data = vec![0u8; tokens_size];
        reader.read_exact(&mut tokens_data)?;
        tokens_data
    };
    let mut num = 0;
    let tokens = {
        let mut tokens = Vec::new();
        let mut current = 0;
        while current < tokens_data.len() {
            if tokens_data[current] != 0 {
                let start = current;
                let end = find_next_null(&tokens_data, start).unwrap();
                let string = str::from_utf8(&tokens_data[start..end])?;
                tokens.push((num, string.to_owned()));
                current = end;
            }
            current += 1;
            num += 1;
        }
        tokens
    };
    Ok((num, tokens))
}

fn get_field<'a>(save_struct: &'a [(&str, Vec<u8>)], field_name: &str) -> Option<&'a Vec<u8>> {
    let bytes = save_struct.iter().find(|(name, _)| *name == field_name).map(|(_, bytes)| bytes)?;
    Some(bytes)
}

fn read_u32_field(save_struct: &[(&str, Vec<u8>)], field_name: &str) -> Option<u32> {
    let field_bytes_source = get_field(save_struct, field_name)?;
    let mut field_bytes = [0u8; 4];
    field_bytes.copy_from_slice(field_bytes_source);
    let connection_count = u32::from_le_bytes(field_bytes);
    Some(connection_count)
}

fn read_str_field<'a>(save_struct: &'a [(&str, Vec<u8>)], field_name: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    let field_bytes = get_field(save_struct, field_name).unwrap();
    let field_str_end = find_next_null(&field_bytes, 0).unwrap_or(field_bytes.len());
    let field_str = str::from_utf8(&field_bytes[0..field_str_end])?;
    Ok(field_str)
}

fn read_str<'a>(bytes: &'a [u8]) -> Result<&'a str, Box<dyn std::error::Error>> {
    let field_str_end = find_next_null(&bytes, 0).unwrap_or(bytes.len());
    let field_str = str::from_utf8(&bytes[0..field_str_end])?;
    Ok(field_str)
}

fn read_u32<'a>(bytes: &'a [u8]) -> Result<u32, Box<dyn std::error::Error>> {
    let mut value_bytes = [0u8; 4];
    value_bytes.copy_from_slice(bytes);
    let value = u32::from_le_bytes(value_bytes);
    Ok(value)
}

fn read_f32<'a>(bytes: &'a [u8]) -> Result<f32, Box<dyn std::error::Error>> {
    let mut value_bytes = [0u8; 4];
    value_bytes.copy_from_slice(bytes);
    let value = f32::from_le_bytes(value_bytes);
    Ok(value)
}

fn record_fields<'a>(fields: &'a [(&str, Vec<u8>)], prefix: &str, output: &mut String) -> Result<(), Box<dyn std::error::Error>> {
    for (field_name, field_data) in fields {
        write!(output, "{}{}: ", prefix, field_name)?;
        match *field_name {
            "classname" | "model" | "message" | "netname" | "targetname" => record_str_field(field_data, output)?,
            "modelindex" | "spawnflags" | "flags" => record_u32_field(field_data, output)?,
            "absmin" | "absmax" | "origin" | "angles" | "v_angle" | "mins" | "maxs" | "view_ofs" | "size" | "m_HackedGunPos" | "movedir" | "m_vecPosition2" => record_vec3_field(field_data, output)?,
            _ => write!(output, "(len: {}) {:02X?}", field_data.len(), field_data)?,
        }
        writeln!(output)?;
    }
    Ok(())
}

fn record_str_field<'a>(field_data: &'a [u8], output: &mut String) -> Result<(), Box<dyn std::error::Error>> {
    let field_str = read_str(field_data)?;
    write!(output, "\"{}\"", field_str)?;
    Ok(())
}

fn record_u32_field<'a>(field_data: &'a [u8], output: &mut String) -> Result<(), Box<dyn std::error::Error>> {
    let value = read_u32(field_data)?;
    write!(output, "{} (0x{:X})", value, value)?;
    Ok(())
}

fn record_f32_field<'a>(field_data: &'a [u8], output: &mut String) -> Result<(), Box<dyn std::error::Error>> {
    let value = read_f32(field_data)?;
    write!(output, "{:.2}", value)?;
    Ok(())
}

fn record_vec3_field<'a>(field_data: &'a [u8], output: &mut String) -> Result<(), Box<dyn std::error::Error>> {
    let x = read_f32(&field_data[..4])?;
    let y = read_f32(&field_data[4..8])?;
    let z = read_f32(&field_data[8..12])?;
    write!(output, "{:.2}, {:.2}, {:.2}", x, y, z)?;
    Ok(())
}