use std::{
    borrow::Cow,
    fmt::Write,
    path::{Path, PathBuf},
};

use gsparser::{
    bsp::{BspEntity, BspReader},
    mdl::null_terminated_bytes_to_str,
    sav::{find_next_null, BytesReader, GameHeader, SavHeader, StringTable},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let sav_path = args.get(0).unwrap();
    let game_root = args.get(1).unwrap();
    let output_path = args.get(2).unwrap();

    let sav_path = PathBuf::from(sav_path);
    let game_root = PathBuf::from(game_root);
    let output_path = PathBuf::from(output_path);

    let sav_paths = if sav_path.is_dir() {
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

        println!(
            "  num_pairs: {}    num_entities: {}    remainder: {}    divided: {}",
            data.num_entries,
            num_entities,
            data.num_entries % num_entities,
            data.num_entries as f64 / num_entities as f64
        );
        //assert!(data.num_pairs % num_entities == 0, "num_pairs: {}    num_entities: {}    remainder: {}    divided: {}", data.num_entries, num_entities, data.num_entries % num_entities, data.num_entries as f64 / num_entities as f64);

        assert!(data.num_world_spawns > 1);
        for window in data.world_spawn_indices.windows(2) {
            let first_index = window[0];
            let second_index = window[1];
            let between_world_spawns = second_index - first_index;
            println!(
                "  Entries between world spawns {} and {}: {}",
                first_index, second_index, between_world_spawns
            );
        }

        /*
        let mut map_entities_iter = entities.iter();
        let mut sav_entities_iter = data.entities.iter();
        while let Some((sav_entity, _)) = sav_entities_iter.next() {
            let should_skip = |class_name: &str| -> bool {
                // Some of these we skip because they are never present at runtime (e.g. lights).
                // Others we skip becuase they aren't represented in the map (e.g. player).
                // The last category we skip are entities that can be removed at runtime (e.g. trigger_once).
                // POSTMORTEM: Nearly every entity can be removed at runtime... this won't work. It's probably the case
                //             that when loading a save file in Half-Life, no entities are spawned via the information
                //             in the bsp. Instead, all information about what entities to spawn and what their properties
                //             are are in the save file. Unless this information is in other parts of the save file...
                match class_name {
                    "light" | "player" | "light_spot" | "info_node" | "trigger_once" | "trigger_auto" | "item_suit" | "func_breakable" | "env_explosion" | "env_shooter" | "scripted_sentence" => true,
                    _ => false,
                }
            };
            if should_skip(&sav_entity) {
                continue;
            }

            let mut found_match = false;
            while let Some(map_entity) = map_entities_iter.next() {
                let map_entity_class_name = map_entity.0["classname"];
                // Skip-able
                if should_skip(&map_entity_class_name) {
                    continue;
                }
                if map_entity_class_name == sav_entity.as_str() {
                    found_match = true;
                    break;
                } else {
                    panic!("Unskippable entity \"{}\" found when looking for \"{}\"!", map_entity_class_name, sav_entity);
                }
            }
            if !found_match {
                panic!("Match not found!");
            }
        }
        */

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

struct SavData {
    map_name: String,
    num_entries: usize,
    num_world_spawns: usize,
    entries: Vec<(usize, String)>,
    world_spawn_indices: Vec<usize>,
    output: String,
    entities: Vec<(String, Vec<(String, Vec<(String, Vec<u8>)>)>)>,
}

fn process_path<P: AsRef<Path>>(sav_path: P) -> Result<SavData, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(sav_path)?;

    let mut output = String::new();

    let reader = BytesReader::new(&bytes);
    // Header
    let sav_header = SavHeader::parse(&reader)?;
    sav_header.record("", &mut output)?;

    // Root string table
    let tokens = StringTable::parse(&reader)?;
    tokens.record("", &mut output)?;
    writeln!(&mut output)?;

    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;

    // Read game header
    let game_header = GameHeader::parse(&reader, &tokens)?;
    game_header.record("", &mut output)?;
    let map_name = game_header.map_name.unwrap();
    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;

    let (_, global_struct) = read_struct(&reader, Some("GLOBAL"), &tokens, &mut output)?;
    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;

    // We should have a 'm_listCount' with the number of door infos
    let list_count = read_u32_field(&global_struct, "m_listCount").unwrap();
    let mut door_infos = Vec::with_capacity(list_count as usize);
    for _ in 0..list_count {
        let (_, gent_struct) = read_struct(&reader, Some("GENT"), &tokens, &mut output)?;

        let name = read_str_field(&gent_struct, "name")?;
        let level_name = read_str_field(&gent_struct, "levelName")?;

        let state_bytes = get_field(&gent_struct, "state");

        door_infos.push((name.to_owned(), level_name.to_owned(), state_bytes.clone()));
    }
    writeln!(&mut output, "Door infos ({}):", list_count)?;
    for (name, level_name, state_bytes) in &door_infos {
        writeln!(
            &mut output,
            "  {:24} ({:10}) ({:02X?})",
            name, level_name, state_bytes
        )?;
    }

    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;

    let hl1_header_start = reader.position();
    let (hl1_name, hl1_header, hl1_block) = read_hl_block(&reader)?;
    let hl1_block_start = hl1_header_start + hl1_header.len() + 4;
    writeln!(&mut output, "HL1 Name: \"{}\"", hl1_name)?;

    let (hl2_name, hl2_header, hl2_block) = read_hl_block(&reader)?;
    writeln!(&mut output, "HL2 Name: \"{}\"", hl2_name)?;

    let (hl3_name, hl3_header, hl3_block) = read_hl_block(&reader)?;
    writeln!(&mut output, "HL3 Name: \"{}\"", hl3_name)?;

    // How many are there?
    let mut num_blocks = 3;
    while (reader.position() as usize) < bytes.len() {
        let (hl1_name, hl1_header, hl1_block) = read_hl_block(&reader)?;
        writeln!(&mut output, "HLX Name: \"{}\"", hl1_name)?;
        num_blocks += 1;
    }
    writeln!(
        &mut output,
        "Num HL blocks: {} (0x{:X})",
        num_blocks, num_blocks
    )?;
    let offset = reader.position();
    writeln!(&mut output, "Current Offset: {} (0x{:X})", offset, offset)?;
    assert_eq!(offset as usize, bytes.len());

    // Poke at the first HL1 block
    let mut hl1_block_reader = std::io::Cursor::new(&hl1_block);
    let hl1_block_reader = BytesReader::new(hl1_block);
    let magic = hl1_block_reader.read(4)?;
    assert_eq!(&magic, b"VALV");
    let version = hl1_block_reader.read_u32_le()?;
    assert_eq!(version, 0x71);
    let unknown_1 = hl1_block_reader.read_u32_le()?;
    writeln!(
        &mut output,
        "  unknown_1: {} (0x{:X})",
        unknown_1, unknown_1
    )?;
    let expected_num_etables = hl1_block_reader.read_u32_le()?;
    writeln!(
        &mut output,
        "  expected_num_etables: {} (0x{:X})",
        expected_num_etables, expected_num_etables
    )?;
    let tokens = StringTable::parse(&hl1_block_reader)?;
    tokens.record("", &mut output)?;

    let offset = hl1_block_reader.position() as usize;
    writeln!(
        &mut output,
        "Current HL1 Block Offset (Relative): {} (0x{:X})",
        offset, offset
    )?;
    writeln!(
        &mut output,
        "Current HL1 Block Offset: {} (0x{:X})",
        hl1_block_start + offset,
        hl1_block_start + offset
    )?;

    let mut num_etables = 0;
    for _ in 0..expected_num_etables {
        let etable_struct = read_struct(&hl1_block_reader, Some("ETABLE"), &tokens, &mut output)?;
        num_etables += 1;
    }
    writeln!(
        &mut output,
        "num_etables: {} ({})",
        num_etables, num_etables
    )?;
    assert_eq!(num_etables, expected_num_etables);

    let (_, save_header) =
        read_struct(&hl1_block_reader, Some("Save Header"), &tokens, &mut output)?;
    let connection_count = read_u32_field(&save_header, "connectionCount").unwrap();
    for _ in 0..connection_count {
        let (_, adjacency_data) =
            read_struct(&hl1_block_reader, Some("ADJACENCY"), &tokens, &mut output)?;
    }

    // Read "LIGHTSTYLE" structs
    let light_style_count = read_u32_field(&save_header, "lightStyleCount").unwrap();
    for _ in 0..light_style_count {
        let (_, light_style) =
            read_struct(&hl1_block_reader, Some("LIGHTSTYLE"), &tokens, &mut output)?;
    }

    // Read "ENTVARS" structs
    let entity_count = read_u32_field(&save_header, "entityCount").unwrap();
    //println!("entity_count: {}", entity_count);
    //println!("num_etables: {}", num_etables);
    let mut current_entity: Option<Vec<(&str, Vec<(&str, &[u8])>)>> = None;
    let mut entities = Vec::with_capacity(entity_count as usize);
    while entities.len() < entity_count as usize {
        //let offset = hl1_block_reader.position() + hl1_block_start;
        //println!("  offset: 0x{:X}", offset);
        let (ty, entity_vars) = match read_struct(&hl1_block_reader, None, &tokens, &mut output) {
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
    for entity in &entities {
        // The first should be ENTVARS which should have a class name
        let class_name = read_str_field(&entity[0].1, "classname")?;
        writeln!(&mut output, "  {}", class_name)?;
        for fragment in entity {
            writeln!(&mut output, "    {}", fragment.0)?;
            record_fields(&fragment.1, "      ", &mut output)?;
        }
    }

    let offset = hl1_block_reader.position() as usize;
    writeln!(
        &mut output,
        "Current HL1 Block Offset (Relative): {} (0x{:X})",
        offset, offset
    )?;
    writeln!(
        &mut output,
        "Current HL1 Block Offset: {} (0x{:X})",
        hl1_block_start + offset,
        hl1_block_start + offset
    )?;

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
                writeln!(
                    &mut output,
                    "WARNING: Assuming false positive at {:X} due to invalid utf8 class name.",
                    current
                )?;
                current += 1;
                continue;
            }
            let class_name = class_name_result?;

            if class_name.len() + 1 != number {
                writeln!(
                    &mut output,
                    "WARNING: Assuming false positive at {:X} due to wrong class name length. ",
                    current
                )?;
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
    assert_eq!(
        first_offset_and_class_name.map(|(_, class_name)| class_name),
        Some("worldspawn")
    );

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

        writeln!(
            &mut output,
            "  {:6X} {:8} {:8} {:8}  {}",
            offset, offset_distance, end_distance, end_to_next_offset, class_name
        )?;
    }

    writeln!(&mut output, "There are {} pairs.", offsets_and_ends.len())?;
    writeln!(
        &mut output,
        "There are {} pairs that have a worldspawn class name",
        world_spawn_indices.len()
    )?;
    // Doesn't work with all my saves...
    //assert!(offsets_and_ends.len() % num_world_spawn == 0, "Number of pairs {} are not divisible by {}", offsets_and_ends.len(), num_world_spawn);

    let num_entries = entries.len();
    let num_world_spawns = world_spawn_indices.len();

    let entities = {
        let mut new_entities = Vec::with_capacity(entities.len());
        for fragments in entities {
            // Read class name
            let class_name = read_str_field(&fragments[0].1, "classname")?.to_owned();

            let mut new_fragments = Vec::with_capacity(fragments.len());
            for (fragment_name, fields) in fragments {
                let mut new_fields = Vec::with_capacity(fields.len());
                for (field_name, field_data) in fields {
                    new_fields.push((field_name.to_owned(), field_data.to_vec()));
                }
                new_fragments.push((fragment_name.to_owned(), new_fields));
            }

            new_entities.push((class_name, new_fragments));
        }
        new_entities
    };

    Ok(SavData {
        map_name: map_name.to_owned(),
        num_entries,
        num_world_spawns,
        entries,
        world_spawn_indices,
        output,
        entities,
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

fn read_struct<'a, 'b>(
    reader: &'b BytesReader<'b>,
    expected_name: Option<&str>,
    string_table: &'a StringTable<'a>,
    output: &mut String,
) -> Result<(&'a str, Vec<(&'a str, &'b [u8])>), Box<dyn std::error::Error>> {
    let always_4 = reader.read_u16_le()?;
    assert_eq!(always_4, 4);
    let token_offset = reader.read_u16_le()?;
    let token = string_table.get(token_offset as u32).unwrap();
    //assert_eq!(token, expected_name);
    if let Some(expected_name) = expected_name {
        if token != expected_name {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Expected \"{}\", found \"{}\"!", expected_name, token),
            )));
        }
    }
    writeln!(output, "\"{}\":", token)?;
    let fields_saved = reader.read_u16_le()?;
    writeln!(output, "  Fields: {} (0x{:X})", fields_saved, fields_saved)?;
    // Not what this short is for
    let unknown = reader.read_u16_le()?;
    assert_eq!(unknown, 0);

    // Read each field
    let mut fields = Vec::with_capacity(fields_saved as usize);
    for _ in 0..fields_saved {
        let payload_size = reader.read_u16_le()?;
        let token_offset = reader.read_u16_le()?;
        let token = string_table.get(token_offset as u32).unwrap();

        let payload = reader.read(payload_size as usize)?;
        fields.push((token, payload));
    }
    for (field_name, payload) in &fields {
        writeln!(output, "    \"{}\" {:02X?}", field_name, payload)?;
    }

    Ok((token, fields))
}

fn read_hl_block<'a>(
    reader: &'a BytesReader<'a>,
) -> Result<(&'a str, &'a [u8], &'a [u8]), Box<dyn std::error::Error>> {
    let hl1_header_len = 260;
    let hl1_header = reader.read(hl1_header_len)?;
    let hl1_name_start = 0;
    let hl1_name_end = find_next_null(&hl1_header, hl1_name_start).unwrap_or(hl1_header.len());
    let hl1_name = str::from_utf8(&hl1_header[hl1_name_start..hl1_name_end])?;

    let hl1_block_len = reader.read_u32_le()?;
    let hl1_block = reader.read(hl1_block_len as usize)?;

    Ok((hl1_name, hl1_header, hl1_block))
}

fn get_field<'a, 'b>(save_struct: &'a [(&str, &'b [u8])], field_name: &str) -> Option<&'b [u8]> {
    let bytes = save_struct
        .iter()
        .find(|(name, _)| *name == field_name)
        .map(|(_, bytes)| bytes)?;
    Some(bytes)
}

fn read_u32_field(save_struct: &[(&str, &[u8])], field_name: &str) -> Option<u32> {
    let field_bytes_source = get_field(save_struct, field_name)?;
    let mut field_bytes = [0u8; 4];
    field_bytes.copy_from_slice(field_bytes_source);
    let connection_count = u32::from_le_bytes(field_bytes);
    Some(connection_count)
}

fn read_str_field<'a>(
    save_struct: &'a [(&str, &[u8])],
    field_name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
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

fn record_fields<'a>(
    fields: &'a [(&str, &[u8])],
    prefix: &str,
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    for (field_name, field_data) in fields {
        write!(output, "{}{}: ", prefix, field_name)?;
        match *field_name {
            "classname" | "model" | "message" | "netname" | "targetname" => {
                record_str_field(field_data, output)?
            }
            "modelindex" | "spawnflags" | "flags" | "skillLevel" | "entityCount" => {
                record_u32_field(field_data, output)?
            }
            "absmin" | "absmax" | "origin" | "angles" | "v_angle" | "mins" | "maxs"
            | "view_ofs" | "size" | "m_HackedGunPos" | "movedir" | "m_vecPosition2"
            | "m_vecAngle2" | "m_vecFinalAngle" => record_vec3_field(field_data, output)?,
            _ => write!(output, "(len: {}) {:02X?}", field_data.len(), field_data)?,
        }
        writeln!(output)?;
    }
    Ok(())
}

fn record_str_field<'a>(
    field_data: &'a [u8],
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let field_str = read_str(field_data)?;
    write!(output, "\"{}\"", field_str)?;
    Ok(())
}

fn record_u32_field<'a>(
    field_data: &'a [u8],
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = read_u32(field_data)?;
    write!(output, "{} (0x{:X})", value, value)?;
    Ok(())
}

fn record_f32_field<'a>(
    field_data: &'a [u8],
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let value = read_f32(field_data)?;
    write!(output, "{:.2}", value)?;
    Ok(())
}

fn record_vec3_field<'a>(
    field_data: &'a [u8],
    output: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
    let x = read_f32(&field_data[..4])?;
    let y = read_f32(&field_data[4..8])?;
    let z = read_f32(&field_data[8..12])?;
    write!(output, "{:.2}, {:.2}, {:.2}", x, y, z)?;
    Ok(())
}


trait SavTestRecord {
    fn record(&self, prefix: &str, output: &mut String) -> std::fmt::Result;
}

impl SavTestRecord for SavHeader {
    fn record(&self, prefix: &str, output: &mut String) -> std::fmt::Result {
        writeln!(output, "{}Header:", prefix)?;
        writeln!(output, "{}  magic: {:X?}", prefix, self.magic)?;
        writeln!(output, "{}  version: 0x{:X}", prefix, self.version)?;
        writeln!(
            output,
            "{}  global_entities_len: {} (0x{:X})",
            prefix, self.global_entities_len, self.global_entities_len
        )?;
        Ok(())
    }
}

impl<'a> SavTestRecord for StringTable<'a> {
    fn record(&self, prefix: &str, output: &mut String) -> std::fmt::Result {
        writeln!(output, "{}String Table ({}):", prefix, self.len())?;
        let keys = self.get_sorted_keys();
        for key in keys {
            let value = self.get(key).unwrap();
            writeln!(output, "{}  ({:4})  \"{}\"", prefix, key, value)?;
        }
        Ok(())
    }
}

impl<'a> SavTestRecord for GameHeader<'a> {
    fn record(&self, prefix: &str, output: &mut String) -> std::fmt::Result {
        writeln!(output, "{}Game Header:", prefix)?;
        if let Some(map_count) = self.map_count {
            writeln!(output, "{}  map_count: {} (0x{:X})", prefix, map_count, map_count)?;
        }
        if let Some(map_name) = self.map_name {
            writeln!(output, "{}  map_name: \"{}\"", prefix, map_name)?;
        }
        if let Some(comment) = self.comment {
            writeln!(output, "{}  comment: \"{}\"", prefix, comment)?;
        }
        Ok(())
    }
}
