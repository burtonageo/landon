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

#[macro_use]
extern crate failure;
use serde;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;

pub use self::export::*;
pub use crate::interpolate::ActionSettings;
pub use crate::interpolate::InterpolationSettings;
use nalgebra::Matrix4;

mod convert;
mod export;
mod interpolate;

/// Something went wrong in the Blender child process that was trying to parse your armature data.
#[derive(Debug, Fail)]
pub enum BlenderError {
    /// Errors in Blender are written to stderr. We capture the stderr from the `blender` child
    /// process that we spawned when attempting to export armature from a `.blend` file.
    #[fail(
        display = "There was an issue while exporting armature: Blender stderr output: {}",
        _0
    )]
    Stderr(String),
}

/// A bone in an armature. Can either be a dual quaternion or a matrix. When you export bones
/// from Blender they come as matrices - BlenderArmature lets you convert them into dual
/// quaternions which are usually more favorable for when implementing skeletal animation.
///
/// TODO: Maybe? Use nalgebra::Matrix4 instead of our arrays. We'd want a custom serializer /
/// deserializer so that we don't need to litter our JSON with `Matrix4` object declarations
/// when we output it from Blender.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub enum Bone {
    // FIXME: Revisit these data types. Either arrays or nalgebra types.. i.e.
    // [Quaternion, Quaternion]
    Matrix([f32; 16]),
    DualQuat([f32; 8]),
}

/// All of the data about a Blender armature that we've exported from Blender.
/// A BlenderArmature should have all of the data that you need to implement skeletal
/// animation.
///
/// If you have other needs, such as a way to know the model space position of any bone at any
/// time so that you can, say, render a baseball in on top of your hand bone.. Open an issue.
/// (I plan to support this specific example in the future)
///
/// TODO: BlenderArmature.y_up() fixes the actions to be y up instead of z up
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(Default, Clone))]
pub struct BlenderArmature {
    pub joint_index: HashMap<String, u8>,
    pub inverse_bind_poses: Vec<Bone>,
    // TODO: Generic type instead of string for your action names so that you can have an enum
    // for your action names ... ?
    // TODO: Inner HashMap should have a float key not a string since it is a time in seconds
    // but you can't have floats as keys so need a workaround.
    // TODO: &str for action name instead of String?
    pub actions: HashMap<String, Vec<Keyframe>>,
}

/// The pose bones at an individual keyframe time
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(Default, Clone))]
pub struct Keyframe {
    frame_time_secs: f32,
    bones: Vec<Bone>,
}

// TODO: These methods can be abstracted into calling a method that takes a callback
impl BlenderArmature {
    /// Iterate over all of the action bones and apply and multiply in the inverse bind pose.
    ///
    /// TODO: another function to apply bind shape matrix? Most armatures seem to export an identity
    /// bind shape matrix but that might not be the same for every armature.
    pub fn apply_inverse_bind_poses(&mut self) {
        for (_name, action) in self.actions.iter_mut() {
            for keyframe in action.iter_mut() {
                for (index, bone) in keyframe.bones.iter_mut().enumerate() {
                    bone.multiply(&mut self.inverse_bind_poses[index]);
                }
            }
        }
    }

    /// Tranpose all of the bone matrices in our armature's action keyframes.
    /// Blender uses row major matrices, but OpenGL uses column major matrices so you'll
    /// usually want to transpose your matrices before using them.
    pub fn transpose_actions(&mut self) {
        for (_name, action) in self.actions.iter_mut() {
            for keyframe in action.iter_mut() {
                for (_index, bone) in keyframe.bones.iter_mut().enumerate() {
                    bone.transpose();
                }
            }
        }
    }
}

impl BlenderArmature {
    /// Convert your action matrices into dual quaternions so that you can implement
    /// dual quaternion linear blending.
    pub fn actions_to_dual_quats(&mut self) {
        for (_, keyframes) in self.actions.iter_mut() {
            for keyframe in keyframes.iter_mut() {
                for bone in keyframe.bones.iter_mut() {
                    *bone = BlenderArmature::matrix_to_dual_quat(bone);
                }
            }
        }
    }
}

impl Bone {
    fn multiply(&mut self, rhs: &mut Bone) {
        match self {
            Bone::Matrix(ref mut lhs_matrix) => match rhs {
                Bone::Matrix(ref mut rhs_matrix) => {
                    let mut lhs_mat4 = Matrix4::identity();
                    lhs_mat4.copy_from_slice(lhs_matrix);

                    let mut rhs_mat4 = Matrix4::identity();
                    rhs_mat4.copy_from_slice(rhs_matrix);

                    let multiplied = rhs_mat4 * lhs_mat4;

                    lhs_matrix.copy_from_slice(multiplied.as_slice());
                }
                Bone::DualQuat(_) => {}
            },
            Bone::DualQuat(_) => {}
        };
    }

    fn transpose(&mut self) {
        match self {
            Bone::Matrix(ref mut matrix) => {
                let mut mat4 = Matrix4::identity();
                mat4.copy_from_slice(matrix);
                mat4.transpose_mut();

                matrix.copy_from_slice(mat4.as_slice());
            }
            Bone::DualQuat(_) => panic!("Cannot transpose dual quat"),
        };
    }

    /// Get a slice representation of you bone data
    ///
    /// Dual Quat -> [Rx, Ry, Rz, Rw, Tx, Ty, Tz, Tw]
    /// Matrix -> [f32; 16]. If from Blender will be row major
    pub fn as_slice(&self) -> &[f32] {
        match self {
            Bone::Matrix(ref matrix) => &matrix[..],
            Bone::DualQuat(ref dual_quat) => &dual_quat[..],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: dual_quat_z_up_to_y_up... but we can just get the rendering working first
    // https://github.com/chinedufn/change-mat4-coordinate-system/blob/master/change-mat4-coordinate-system.js
    #[test]
    fn applying_inv_bind_poses() {
        let mut start_actions = HashMap::new();
        let mut keyframes = vec![];
        keyframes.push(Keyframe {
            frame_time_secs: 1.0,
            bones: vec![Bone::Matrix([
                1.0, 6.0, 2.0, 1.0, 7.0, 1.0, 2.0, 5.0, 0.0, 4.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ])],
        });
        start_actions.insert("Fly".to_string(), keyframes);

        let mut start_armature = BlenderArmature {
            actions: start_actions,
            inverse_bind_poses: vec![Bone::Matrix([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0, 1.0,
            ])],
            ..BlenderArmature::default()
        };

        start_armature.apply_inverse_bind_poses();

        let mut end_actions = HashMap::new();
        let mut keyframes = vec![];
        keyframes.push(Keyframe {
            frame_time_secs: 1.0,
            bones: vec![Bone::Matrix([
                1.0, 6.0, 7.0, 1.0, 7.0, 1.0, 27.0, 5.0, 0.0, 4.0, 1.0, 0.0, 0.0, 0.0, 5.0, 1.0,
            ])],
        });
        end_actions.insert("Fly".to_string(), keyframes);

        let expected_armature = BlenderArmature {
            actions: end_actions,
            ..start_armature.clone()
        };

        assert_eq!(start_armature, expected_armature);
    }

    #[test]
    fn convert_actions_to_dual_quats() {
        let mut start_actions = HashMap::new();
        let mut keyframes = vec![];
        keyframes.push(Keyframe {
            frame_time_secs: 1.0,
            bones: vec![Bone::Matrix([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ])],
        });
        start_actions.insert("Fly".to_string(), keyframes);

        let mut start_armature = BlenderArmature {
            actions: start_actions,
            ..BlenderArmature::default()
        };

        start_armature.actions_to_dual_quats();

        let mut end_actions = HashMap::new();
        let mut keyframes = vec![];
        keyframes.push(Keyframe {
            frame_time_secs: 1.0,
            bones: vec![Bone::DualQuat([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])],
        });
        end_actions.insert("Fly".to_string(), keyframes);

        let expected_armature = BlenderArmature {
            actions: end_actions,
            ..start_armature.clone()
        };

        assert_eq!(start_armature, expected_armature);
    }

    // TODO: Function to return these start_actions that we keep using
    #[test]
    fn transpose_actions() {
        let mut start_actions = HashMap::new();
        let mut keyframes = vec![];
        keyframes.push(Keyframe {
            frame_time_secs: 1.0,
            bones: vec![Bone::Matrix([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0, 1.0,
            ])],
        });

        start_actions.insert("Fly".to_string(), keyframes);

        let mut start_armature = BlenderArmature {
            actions: start_actions,
            ..BlenderArmature::default()
        };

        start_armature.transpose_actions();

        let mut end_actions = HashMap::new();
        let mut keyframes = vec![];
        keyframes.push(Keyframe {
            frame_time_secs: 1.0,
            bones: vec![Bone::Matrix([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 5.0, 0.0, 0.0, 0.0, 1.0,
            ])],
        });
        end_actions.insert("Fly".to_string(), keyframes);

        let expected_armature = BlenderArmature {
            actions: end_actions,
            ..start_armature.clone()
        };

        assert_eq!(start_armature, expected_armature);
    }
}
