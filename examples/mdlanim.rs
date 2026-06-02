use std::{collections::HashMap, path::PathBuf};

use gsparser::mdl::MdlFile;
use gsparser::steam::get_half_life_steam_install_path;
use gsparser::util::resolve_null_terminated_string;
use id_tree::InsertBehavior::AsRoot;
use id_tree::InsertBehavior::UnderNode;
use id_tree::TreeBuilder;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let game_root = if args.len() == 1 {
        let game_root = PathBuf::from(args.get(0).unwrap());
        game_root
    } else {
        // Infer the game root via Steam
        let game_root =
            get_half_life_steam_install_path().expect("Failed to find Half-Life install location!");
        game_root
    };

    let models_path = {
        let mut path = PathBuf::from(game_root);
        path.push("models");
        path
    };

    /*
    // Collect weapon models
    let mut model_paths = Vec::new();
    for entry in std::fs::read_dir(models_path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let entry_path = entry.path();
            let extension = entry_path.extension().unwrap().to_str().unwrap();
            if extension == "mdl" {
                let file_stem = entry_path.file_stem().unwrap().to_str().unwrap();
                //if file_stem.starts_with("v_") {
                //    model_paths.push((file_stem.to_owned(), entry_path));
                //}
                if file_stem.chars().nth(1).unwrap() != '_'
                    && !file_stem.chars().rev().next().unwrap().is_numeric()
                {
                    model_paths.push((file_stem.to_owned(), entry_path));
                }
            }
        }
    }
    */

    // Collect models
    let mut model_paths = Vec::new();
    for entry in std::fs::read_dir(models_path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let entry_path = entry.path();
            let extension = entry_path.extension().unwrap().to_str().unwrap();
            if extension == "mdl" {
                let file_stem = entry_path.file_stem().unwrap().to_str().unwrap();
                if !file_stem.chars().rev().next().unwrap().is_numeric() {
                    model_paths.push((file_stem.to_owned(), entry_path));
                }
            }
        }
    }

    // Inspect model animations
    for (name, path) in model_paths {
        println!("{}", name);
        if let Ok(file) = MdlFile::open(path) {
            //for animation in &file.animations {
            //    println!("  {}", animation.name);
            //    for (i, bone_animation) in animation.bone_animations.iter().enumerate() {
            //        println!("    {} - target: {}", i, bone_animation.target);
            //        for channel in &bone_animation.channels {
            //            println!("      {:?}", channel.target);
            //        }
            //    }
            //}

            /*
            if file.bones.len() > 0 {
                //println!("{}", name);
                let num_roots: usize = file
                    .bones
                    .iter()
                    .map(|x| if x.parent < 0 { 1 } else { 0 })
                    .sum();
                //println!("{} - {}", name, num_roots);
                let mut bone_tree = TreeBuilder::new()
                    .with_node_capacity(file.bones.len())
                    .build();
                let mut bone_map = HashMap::new();
                for (i, bone) in file.bones.iter().enumerate() {
                    //println!("Bone {} : Parnet {}", i, bone.parent);
                    let behavior = if bone.parent < 0 {
                        AsRoot
                    } else {
                        let parent_node = bone_map.get(&(bone.parent as usize)).unwrap();
                        UnderNode(parent_node)
                    };
                    let name = resolve_null_terminated_string(&bone.name);
                    let bone_id = bone_tree
                        .insert(id_tree::Node::new(name.to_string()), behavior)
                        .unwrap();
                    bone_map.insert(i, bone_id);
                }

                let mut text = String::new();
                bone_tree.write_formatted(&mut text).unwrap();

                std::fs::write(format!("testoutput/modeltrees/{}.txt", name), text).unwrap();
            }
            */

            for animation in &file.animations {
                println!("  {}", animation.name);
                let mut animation_len = None;
                for (i, bone_animation) in animation.bone_animations.iter().enumerate() {
                    println!("    {} - target: {}", i, bone_animation.target);
                    for channel in &bone_animation.channels {
                        if let Some(animation_len) = animation_len {
                            assert_eq!(animation_len, channel.keyframes.len());
                        } else {
                            animation_len = Some(channel.keyframes.len());
                        }
                    }
                }
            }
        }
    }
}
