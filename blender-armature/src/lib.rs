//! Blender files can have armature such as circles, cubes, cylinders, a dragon or any other
//! 3D shape.
//!
//! A armature can be represented as a group of vertices and data about those vertices, such as their
//! normals or UV coordinates.
//!
//! Armaturees can also have metadata, such as the name of it's parent armature (useful for vertex
//! skinning).
//!
//! blender-armature-to-json seeks to be a well tested, well documented exporter for blender armature
//! metadata.
//!
//! You can write data to stdout or to a file. At the onset it will be geared towards @chinedufn's
//! needs - but if you have needs that aren't met feel very free to open an issue.
//!
//! @see https://docs.blender.org/manual/en/dev/modeling/armature/introduction.html - Armature Introduction
//! @see https://github.com/chinedufn/blender-actions-to-json - Exporting blender armatures / actions

// TODO: - breadcrumb -> convert this file into blender armature.. and add armature export
// to letter_f.rs test and verify that it matches the expected BlenderArmature

#[macro_use]
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use serde_json::Error;
use std::cmp::max;
use std::collections::HashMap;
use std::collections::HashSet;

/// Something went wrong in the Blender child process that was trying to parse your armature data.
#[derive(Debug, Fail)]
pub enum BlenderError {
    /// Errors in Blender are written to stderr. We capture the stderr from the `blender` child
    /// process that we spawned when attempting to export armature from a `.blend` file.
    #[fail(display = "There was an issue while exporting armature: Blender stderr output: {}", _0)]
    Stderr(String),
}

/// All of the data about a Blender armature
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(Default))]
pub struct BlenderArmature {
    pub jointIndex: HashMap<String, u8>,
    pub inverseBindPoses: Vec<Vec<f32>>,
    pub actions: HashMap<String, HashMap<String, Vec<Vec<f32>>>>,
}

impl BlenderArmature {
    pub fn from_json(json_str: &str) -> Result<BlenderArmature, Error> {
        serde_json::from_str(json_str)
    }
}

pub type ArmatureNamesToData = HashMap<String, BlenderArmature>;
pub type FilenamesToArmaturees = HashMap<String, ArmatureNamesToData>;

/// Given a buffer of standard output from Blender we parse all of the armature JSON that was
/// written to stdout by `blender-armature-to-json.py`.
///
/// Armaturees data in stdout will look like:
///
/// START_ARMATURE_JSON /path/to/file.blend my_armature_name
/// {...}
/// END_ARMATURE_JSON /path/to/file.blend my_armature_name
///
/// @see blender-armature-to-json.py - This is where we write to stdout
pub fn parse_armatures_from_blender_stdout(
    blender_stdout: &str,
) -> Result<FilenamesToArmaturees, failure::Error> {
    let mut filenames_to_armature = HashMap::new();

    let mut index = 0;

    while let Some((filename_to_armature, next_start_index)) =
        find_first_armature_after_index(blender_stdout, index)
    {
        filenames_to_armature.extend(filename_to_armature);
        index = next_start_index;
    }

    // TODO: Breadcrumb - Plan armature visualizer to visualizer our basic_cube.rs on paper.
    // Step 1 is adding a function to our main crate that expands our 3 vertex indices into just one.
    // Unit test it

    Ok(filenames_to_armature)
}

fn find_first_armature_after_index(
    blender_stdout: &str,
    index: usize,
) -> Option<(FilenamesToArmaturees, usize)> {
    let blender_stdout = &blender_stdout[index as usize..];

    if let Some(armature_start_index) = blender_stdout.find("START_ARMATURE_JSON") {
        let mut filenames_to_armature = HashMap::new();
        let mut armature_name_to_data = HashMap::new();

        let armature_end_index = blender_stdout.find("END_ARMATURE_JSON").unwrap();

        let armature_data = &blender_stdout[armature_start_index..armature_end_index];

        let mut lines = armature_data.lines();

        let first_line = lines.next().unwrap();

        let armature_filename: Vec<&str> = first_line.split(" ").collect();
        let armature_filename = armature_filename[1].to_string();

        let armature_name = first_line.split(" ").last().unwrap().to_string();

        let armature_data: String = lines.collect();
        let armature_data: BlenderArmature = serde_json::from_str(&armature_data).unwrap();

        armature_name_to_data.insert(armature_name, armature_data);
        filenames_to_armature.insert(armature_filename, armature_name_to_data);

        return Some((filenames_to_armature, armature_end_index + 1));
    }

    return None;
}

#[cfg(test)]
mod tests {
    use super::*;
}
