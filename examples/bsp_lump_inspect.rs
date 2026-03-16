use gsparser::bsp::{
    BspReader, LUMP_CLIPNODES, LUMP_EDGES, LUMP_ENTITIES, LUMP_FACES, LUMP_LEAVES, LUMP_LIGHTING,
    LUMP_MARKSURFACES, LUMP_MODELS, LUMP_NODES, LUMP_PLANES, LUMP_SURFEDGES, LUMP_TEXINFO,
    LUMP_TEXTURES, LUMP_VERTICES, LUMP_VISIBILITY,
};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = args.get(0).expect("Expected import path!");

    let file_bytes = std::fs::read(path).expect("Failed to open file!");
    let reader = BspReader::read(file_bytes);

    let header = reader.header();
    println!("Header:");
    println!("{:#?}", header);
    println!();
    println!("Lumps:");
    for (i, lump) in header.lumps.iter().enumerate() {
        let lump_name = get_lump_name_for_index(i).unwrap();
        println!("  {}:", lump_name);
        println!("    offset: {}", lump.offset);
        println!("    len:    {}", lump.len);
    }
    println!();
    println!("Present lumps:");
    for (i, lump) in header.lumps.iter().enumerate() {
        if lump.len > 0 {
            let lump_name = get_lump_name_for_index(i).unwrap();
            println!("  {}", lump_name);
        }
    }
}

fn get_lump_name_for_index(index: usize) -> Option<&'static str> {
    let text = match index {
        LUMP_ENTITIES => "Entities",
        LUMP_PLANES => "Planes",
        LUMP_TEXTURES => "Textures",
        LUMP_VERTICES => "Vertices",
        LUMP_VISIBILITY => "Visibility",
        LUMP_NODES => "Nodes",
        LUMP_TEXINFO => "Texture Infos",
        LUMP_FACES => "Faces",
        LUMP_LIGHTING => "Lighting",
        LUMP_CLIPNODES => "Clip Nodes",
        LUMP_LEAVES => "Leaves",
        LUMP_MARKSURFACES => "Mark Surfaces",
        LUMP_EDGES => "Edges",
        LUMP_SURFEDGES => "Surface Edges",
        LUMP_MODELS => "Models",
        _ => return None,
    };
    Some(text)
}
