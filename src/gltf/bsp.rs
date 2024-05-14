use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
};

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use gsparser::{
    bsp::{BspEdge, BspEntity, BspFace, BspReader, BspSurfaceEdge, BspTextureInfo, BspVertex},
    wad3::{MipmapedTextureData, WadArchive, WadFileInfo},
};

use super::{
    animation::Animations,
    buffer::{BufferViewAndAccessorPair, BufferViewTarget, BufferWriter},
    coordinates::convert_coordinates,
    export::write_gltf,
    material::{Image, MagFilter, Material, MaterialData, MinFilter, Texture, Wrap},
    node::{MeshIndex, Node, Nodes},
    skin::Skins,
    Mesh, Model, Vertex, VertexAttributesSource,
};

const QUIVER_PREFIX: &'static str = "\\quiver\\";

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ModelVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex for ModelVertex {
    fn write_slices(
        writer: &mut super::buffer::BufferWriter,
        vertices: &[Self],
    ) -> Box<dyn super::VertexAttributesSource> {
        // Split out the vertex data
        let mut positions = Vec::with_capacity(vertices.len());
        let mut normals = Vec::with_capacity(vertices.len());
        let mut uvs = Vec::with_capacity(vertices.len());
        for vertex in vertices {
            positions.push(vertex.pos);
            normals.push(vertex.normal);
            uvs.push(vertex.uv);
        }

        let vertex_positions_pair = writer
            .create_view_and_accessor_with_min_max(&positions, Some(BufferViewTarget::ArrayBuffer));
        let vertex_normals_pair = writer
            .create_view_and_accessor_with_min_max(&normals, Some(BufferViewTarget::ArrayBuffer));
        let vertex_uvs_pair =
            writer.create_view_and_accessor_with_min_max(&uvs, Some(BufferViewTarget::ArrayBuffer));

        Box::new(DebugVertexAttributes {
            positions: vertex_positions_pair,
            normals: vertex_normals_pair,
            uvs: vertex_uvs_pair,
        })
    }
}

struct DebugVertexAttributes {
    positions: BufferViewAndAccessorPair,
    normals: BufferViewAndAccessorPair,
    uvs: BufferViewAndAccessorPair,
}

impl VertexAttributesSource for DebugVertexAttributes {
    fn attribute_pairs(&self) -> Vec<(&'static str, usize)> {
        vec![
            ("POSITION", self.positions.accessor.0),
            ("NORMAL", self.normals.accessor.0),
            ("TEXCOORD_0", self.uvs.accessor.0),
        ]
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
struct SharedVertex {
    vertex: usize,
    texture: usize,
}

pub struct TextureInfo {
    pub name: String,
    pub image_data: MipmapedTextureData,
}

impl TextureInfo {
    fn new(name: String, image_data: MipmapedTextureData) -> Self {
        Self { name, image_data }
    }
}

pub fn export<P: AsRef<Path>, T: AsRef<Path>>(
    game_root: T,
    reader: &BspReader,
    export_file_path: P,
    mut log: Option<&mut String>,
) -> std::io::Result<()> {
    if let Some(log) = &mut log {
        log_bsp(reader, log);
    }

    let game_root = game_root.as_ref();

    let mut wad_resources = WadCollection::new();
    read_wad_resources(reader, game_root, &mut wad_resources);

    let textures = read_textures(reader, &wad_resources);
    let model = convert(reader, &textures);

    let mut buffer_writer = BufferWriter::new();

    let mut material_data = MaterialData::new();
    let sampler = material_data.add_sampler(super::material::Sampler {
        mag_filter: MagFilter::Linear,
        min_filter: MinFilter::LinearMipMapLinear,
        wrap_s: Wrap::MirroredRepeat,
        wrap_t: Wrap::MirroredRepeat,
    });
    for texture in &textures {
        let image = material_data.add_images(Image {
            uri: format!("{}.png", &texture.name),
        });
        let texture = material_data.add_texture(Texture {
            sampler,
            source: image,
        });
        material_data.add_material(Material {
            base_color_texture: Some(texture),
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            ..Default::default()
        });
    }

    let skins = Skins::new();
    let animations = Animations::new(0);
    let mut nodes = Nodes::new(1);
    let scene_root = nodes.add_node(Node {
        mesh: Some(MeshIndex(0)),
        ..Default::default()
    });

    let buffer_name = "data.bin";
    let gltf_text = write_gltf(
        buffer_name,
        &mut buffer_writer,
        &model,
        &material_data,
        scene_root,
        &nodes,
        &skins,
        &animations,
    );

    let path = export_file_path.as_ref();
    let data_path = if let Some(parent_path) = path.parent() {
        let mut data_path = parent_path.to_owned();
        data_path.push(buffer_name);
        data_path
    } else {
        PathBuf::from(buffer_name)
    };

    std::fs::write(path, gltf_text)?;
    std::fs::write(data_path, buffer_writer.to_inner())?;

    // Write textures
    let mut texture_path = if let Some(parent_path) = path.parent() {
        let mut data_path = parent_path.to_owned();
        data_path.push("something");
        data_path
    } else {
        PathBuf::from("something")
    };
    for texture in textures {
        texture_path.set_file_name(format!("{}.png", &texture.name));
        texture
            .image_data
            .image
            .save_with_format(&texture_path, image::ImageFormat::Png)
            .unwrap();
    }

    Ok(())
}

pub fn read_wad_resources<P: AsRef<Path>>(
    reader: &BspReader,
    game_root: P,
    wad_resources: &mut WadCollection,
) {
    let entities = BspEntity::parse_entities(reader.read_entities());
    let game_root = game_root.as_ref();
    for entity in &entities {
        if let Some(value) = entity.0.get("wad") {
            for wad_path in value.split(';') {
                assert!(wad_path.starts_with(QUIVER_PREFIX));
                let wad_path = &wad_path[QUIVER_PREFIX.len()..];
                let mut path = game_root.to_owned();
                path.push(wad_path);
                let archive = WadArchive::open(path);
                wad_resources.add(archive);
            }
        }
    }
}

pub fn read_textures(reader: &BspReader, wad_resources: &WadCollection) -> Vec<TextureInfo> {
    let texture_reader = reader.read_textures();
    let mut textures = Vec::with_capacity(texture_reader.len());
    for i in 0..texture_reader.len() {
        let reader = texture_reader.get(i).unwrap();
        if reader.has_local_image_data() {
            unimplemented!("bsp local image data not implemented");
        } else {
            let name = reader.get_image_name();
            let search_name = name.to_uppercase();
            let texture_data =
                if let Some((archive, file)) = wad_resources.find(search_name.as_str()) {
                    //println!("Found \"{}\"!", name);
                    let texture_data = archive.decode_mipmaped_image(file);
                    Some(texture_data)
                } else {
                    println!("Couldn't find \"{}\"", name);
                    None
                };
            textures.push(TextureInfo::new(name.to_owned(), texture_data.unwrap()));
        }
    }
    textures
}

pub fn convert(reader: &BspReader, textures: &[TextureInfo]) -> Model<ModelVertex> {
    let mut indices = Vec::new();
    let mut vertices = Vec::new();
    let mut meshes = Vec::new();

    let bsp_vertices = reader.read_vertices();
    let texture_infos = reader.read_texture_infos();

    let mut vertex_map = HashMap::<SharedVertex, usize>::new();
    let mark_surfaces = reader.read_mark_surfaces();
    let faces = reader.read_faces();
    let edges = reader.read_edges();
    let surface_edges = reader.read_surface_edges();
    let planes = reader.read_planes();
    let read_vertex_index = |edge_index: &BspSurfaceEdge, edges: &[BspEdge]| -> u32 {
        let edge_vertex_index: usize = if edge_index.0 > 0 { 0 } else { 1 };
        let edge_index = edge_index.0.abs() as usize;
        let edge = &edges[edge_index];
        edge.vertices[edge_vertex_index] as u32
    };
    for leaf in reader.read_leaves().iter() {
        let mark_surfaces_range =
            leaf.first_mark_surface..leaf.first_mark_surface + leaf.mark_surfaces;
        for mark_surface_index in mark_surfaces_range {
            let mark_surface = &mark_surfaces[mark_surface_index as usize];
            let face = &faces[mark_surface.0 as usize];

            if face.texture_info == 0 {
                continue;
            }

            let surface_edges_range =
                face.first_edge as usize..face.first_edge as usize + face.edges as usize;
            let surface_edges = &surface_edges[surface_edges_range];

            let plane = &planes[face.plane as usize];

            let first_vertex = read_vertex_index(&surface_edges[0], edges);

            let mut triangle_list = Vec::new();
            let to_shared_vertex = |index: u32, face: &BspFace| -> SharedVertex {
                SharedVertex {
                    vertex: index as usize,
                    texture: face.texture_info as usize,
                }
            };
            for i in 0..surface_edges.len() - 2 {
                triangle_list.push(to_shared_vertex(
                    read_vertex_index(&surface_edges[i + 2], edges),
                    face,
                ));
                triangle_list.push(to_shared_vertex(
                    read_vertex_index(&surface_edges[i + 1], edges),
                    face,
                ));
                triangle_list.push(to_shared_vertex(first_vertex, face));
            }
            let start = indices.len();
            process_indexed_triangles(
                &triangle_list,
                face,
                plane.normal,
                bsp_vertices,
                &textures,
                texture_infos,
                &mut indices,
                &mut vertices,
                &mut vertex_map,
            );
            let end = indices.len();

            meshes.push(Mesh {
                indices_range: start..end,
                texture_index: texture_infos[face.texture_info as usize].texture_index as usize,
            });
        }
    }

    Model {
        indices,
        vertices,
        meshes,
    }
}

fn process_indexed_triangles(
    triangle_list: &[SharedVertex],
    face: &BspFace,
    normal: [f32; 3],
    bsp_vertices: &[BspVertex],
    textures: &[TextureInfo],
    texture_infos: &[BspTextureInfo],
    indices: &mut Vec<u32>,
    vertices: &mut Vec<ModelVertex>,
    vertex_map: &mut HashMap<SharedVertex, usize>,
) {
    assert!(
        triangle_list.len() % 3 == 0,
        "Vertices are not a multiple of 3: {}",
        triangle_list.len()
    );
    let texture_info = &texture_infos[face.texture_info as usize];
    let texture = &textures[texture_info.texture_index as usize].image_data;
    let mut process_trivert = |trivert| {
        let index = if let Some(index) = vertex_map.get(trivert) {
            *index
        } else {
            let pos = convert_coordinates(bsp_vertices[trivert.vertex as usize].to_array());
            let pos_vec = Vec3::from_array(pos);
            let s = Vec3::from_array(convert_coordinates(texture_info.s));
            let t = Vec3::from_array(convert_coordinates(texture_info.t));
            let uv = [
                pos_vec.dot(s) + texture_info.s_shift,
                pos_vec.dot(t) + texture_info.t_shift,
            ];

            let normal = convert_coordinates(normal);

            let uv = [
                uv[0] / texture.image_width as f32,
                uv[1] / texture.image_height as f32,
            ];

            let index = vertices.len();
            vertices.push(ModelVertex { pos, normal, uv });
            vertex_map.insert(*trivert, index);
            index
        };
        indices.push(index as u32);
    };

    for trivert in triangle_list {
        process_trivert(trivert);
    }
}

pub struct WadCollection {
    wads: Vec<WadArchive>,
}

impl WadCollection {
    pub fn new() -> Self {
        Self { wads: Vec::new() }
    }

    pub fn add(&mut self, archive: WadArchive) {
        self.wads.push(archive);
    }

    pub fn find(&self, key: &str) -> Option<(&WadArchive, &WadFileInfo)> {
        for wad in &self.wads {
            if let Some(file) = wad.files.iter().find(|x| x.name.as_str() == key) {
                return Some((wad, file));
            }
        }
        None
    }
}

fn log_bsp(reader: &BspReader, log: &mut String) {
    writeln!(log, "Nodes:").unwrap();
    for (i, node) in reader.read_nodes().iter().enumerate() {
        writeln!(log, "  Node {}", i).unwrap();
        writeln!(log, "    plane: {}", node.plane).unwrap();
        writeln!(
            log,
            "    children: [ {}, {} ]",
            node.children[0], node.children[1]
        )
        .unwrap();
        writeln!(
            log,
            "    mins: [ {}, {}, {} ]",
            node.mins[0], node.mins[1], node.mins[2]
        )
        .unwrap();
        writeln!(
            log,
            "    maxs: [ {}, {}, {} ]",
            node.maxs[0], node.maxs[1], node.maxs[2]
        )
        .unwrap();
        writeln!(log, "    first_face: {}", node.first_face).unwrap();
        writeln!(log, "    faces: {}", node.faces).unwrap();
    }

    writeln!(log, "Leaves:").unwrap();
    for (i, leaf) in reader.read_leaves().iter().enumerate() {
        writeln!(log, "  Leaf {}", i).unwrap();
        writeln!(log, "    contents: {:?}", leaf.contents).unwrap();
        writeln!(log, "    vis_offset: {}", leaf.vis_offset).unwrap();
        writeln!(
            log,
            "    mins: [ {}, {}, {} ]",
            leaf.mins[0], leaf.mins[1], leaf.mins[2]
        )
        .unwrap();
        writeln!(
            log,
            "    maxs: [ {}, {}, {} ]",
            leaf.maxs[0], leaf.maxs[1], leaf.maxs[2]
        )
        .unwrap();
        writeln!(log, "    first_mark_surface: {}", leaf.first_mark_surface).unwrap();
        writeln!(log, "    mark_surfaces: {}", leaf.mark_surfaces).unwrap();
        writeln!(
            log,
            "    ambient_levels: [ {}, {}, {}, {} ]",
            leaf.ambient_levels[0],
            leaf.ambient_levels[1],
            leaf.ambient_levels[2],
            leaf.ambient_levels[3]
        )
        .unwrap();
    }

    writeln!(log, "Mark Surfaces:").unwrap();
    for (i, surface) in reader.read_mark_surfaces().iter().enumerate() {
        writeln!(log, "  Mark Surface {}", i).unwrap();
        writeln!(log, "    index: {}", surface.0).unwrap();
    }

    writeln!(log, "Surface Edges:").unwrap();
    for (i, edge) in reader.read_surface_edges().iter().enumerate() {
        writeln!(log, "  Surface Edge {}", i).unwrap();
        writeln!(log, "    index: {}", edge.0).unwrap();
    }

    writeln!(log, "Faces:").unwrap();
    for (i, face) in reader.read_faces().iter().enumerate() {
        writeln!(log, "  Face {}", i).unwrap();
        writeln!(log, "    plane: {}", face.plane).unwrap();
        writeln!(log, "    plane_side: {}", face.plane_side).unwrap();
        writeln!(log, "    first_edge: {}", face.first_edge).unwrap();
        writeln!(log, "    edges: {}", face.edges).unwrap();
        writeln!(log, "    texture_info: {}", face.texture_info).unwrap();
        writeln!(
            log,
            "    styles: [ {}, {}, {}, {} ]",
            face.styles[0], face.styles[1], face.styles[2], face.styles[3]
        )
        .unwrap();
        writeln!(log, "    lightmap_offset: {}", face.lightmap_offset).unwrap();
    }

    writeln!(log, "Edges:").unwrap();
    for (i, edge) in reader.read_edges().iter().enumerate() {
        writeln!(log, "  Edge {}", i).unwrap();
        writeln!(
            log,
            "    vertices: [ {}, {} ]",
            edge.vertices[0], edge.vertices[1]
        )
        .unwrap();
    }

    writeln!(log, "Vertices:").unwrap();
    for (i, vertex) in reader.read_vertices().iter().enumerate() {
        writeln!(log, "  Vertex {}", i).unwrap();
        writeln!(
            log,
            "    vertices: [ {}, {}, {} ]",
            vertex.x, vertex.y, vertex.z
        )
        .unwrap();
    }

    writeln!(log, "Textures:").unwrap();
    let texture_reader = reader.read_textures();
    for i in 0..texture_reader.len() {
        let reader = texture_reader.get(i).unwrap();
        let name = reader.get_image_name();
        writeln!(
            log,
            "  {} - {} - {}",
            i,
            name,
            reader.has_local_image_data()
        )
        .unwrap();
    }

    let entities = reader.read_entities();
    let entities = BspEntity::parse_entities(entities);
    writeln!(log, "Entities:").unwrap();
    for (i, entity) in entities.iter().enumerate() {
        writeln!(log, "  Entity {}", i).unwrap();
        for (key, value) in &entity.0 {
            writeln!(log, "    {}: {}", key, value).unwrap();
        }
    }

    writeln!(log, "Models:").unwrap();
    let models = reader.read_models();
    for (i, model) in models.iter().enumerate() {
        writeln!(log, "  Model {}", i).unwrap();
        writeln!(
            log,
            "    mins: [ {}, {}, {} ]",
            model.mins[0], model.mins[1], model.mins[2]
        )
        .unwrap();
        writeln!(
            log,
            "    maxs: [ {}, {}, {} ]",
            model.maxs[0], model.maxs[1], model.maxs[2]
        )
        .unwrap();
        writeln!(
            log,
            "    origin: [ {}, {}, {} ]",
            model.origin[0], model.origin[1], model.origin[2]
        )
        .unwrap();
        writeln!(
            log,
            "    head_nodes: [ {}, {}, {}, {} ]",
            model.head_nodes[0], model.head_nodes[1], model.head_nodes[2], model.head_nodes[3]
        )
        .unwrap();
        writeln!(log, "    vis_leaves: {}", model.vis_leaves).unwrap();
        writeln!(log, "    first_face: {}", model.first_face).unwrap();
        writeln!(log, "    faces: {}", model.faces).unwrap();
    }
}
