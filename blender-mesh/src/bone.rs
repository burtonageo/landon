use crate::BlenderMesh;

/// The number of bones that influence each uniform.
///
/// When exported from Blender this is non uniform, but becomes uniform when
/// we call `.set_groups_per_vertex` to make every vertex have the same number
/// of influences.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum BoneInfluencesPerVertex {
    NonUniform(Vec<u8>),
    Uniform(u8),
}

impl Default for BoneInfluencesPerVertex {
    fn default() -> Self {
        BoneInfluencesPerVertex::Uniform(0)
    }
}

impl From<Vec<u8>> for BoneInfluencesPerVertex {
    fn from(influences: Vec<u8>) -> Self {
        BoneInfluencesPerVertex::NonUniform(influences)
    }
}

impl BlenderMesh {
    /// Different vertices might have different numbers of bones that influence them.
    /// A vertex near the shoulder might be influenced by the neck and upper arm and sternum,
    /// while a vertex in a toe might only be influenced by a toe bone.
    ///
    /// When passing data to the GPU, each vertex needs the same number of bone attributes, so
    /// we must add/remove bones from each vertex to get them equal.
    ///
    /// Say we're setting 3 groups per vertex:
    ///  - If a vertex has one vertex group (bone) we will create two fake bones with 0.0 weight.
    ///  - If a vertex has 5 bones we'll remove the one with the smallest weighting (influence).
    pub(crate) fn set_bone_influences_per_vertex(&mut self, count: u8) {
        let mut normalized_group_indices = vec![];
        let mut normalized_group_weights = vec![];

        let mut current_index: u32 = 0;

        {
            let indices = self.vertex_group_indices.as_mut().unwrap();
            let weights = self.vertex_group_weights.as_mut().unwrap();

            if let Some(BoneInfluencesPerVertex::NonUniform(bone_influences_per_vertex)) =
                &self.bone_influences_per_vertex
            {
                for group_count in bone_influences_per_vertex.iter() {
                    let mut vertex_indices = vec![];
                    let mut vertex_weights = vec![];

                    for index in current_index..(current_index + *group_count as u32) {
                        vertex_indices.push(index);
                        vertex_weights.push(weights[index as usize]);
                    }

                    vertex_weights.sort_by(|a, b| b.partial_cmp(a).unwrap());
                    vertex_indices.sort_by(|a, b| {
                        weights[*b as usize]
                            .partial_cmp(&weights[*a as usize])
                            .unwrap()
                    });

                    let mut vertex_indices: Vec<u8> = vertex_indices
                        .iter()
                        .map(|i| indices[*i as usize])
                        .collect();

                    vertex_indices.resize(count as usize, 0);
                    vertex_weights.resize(count as usize, 0.0);

                    normalized_group_indices.append(&mut vertex_indices);
                    normalized_group_weights.append(&mut vertex_weights);

                    current_index += *group_count as u32;
                }
            }
        }

        self.bone_influences_per_vertex = Some(BoneInfluencesPerVertex::Uniform(count));

        self.vertex_group_indices = Some(normalized_group_indices);
        self.vertex_group_weights = Some(normalized_group_weights);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_joints_per_vert() {
        let mut start_mesh = BlenderMesh {
            vertex_group_indices: Some(vec![0, 2, 3, 4, 0, 1, 3, 2]),
            bone_influences_per_vertex: Some(vec![1, 3, 4].into()),
            vertex_group_weights: Some(vec![1.0, 0.5, 0.2, 0.3, 0.6, 0.15, 0.1, 0.15]),
            ..BlenderMesh::default()
        };

        start_mesh.set_bone_influences_per_vertex(3);
        let three_joints_per_vert = start_mesh;

        let expected_mesh = BlenderMesh {
            vertex_group_indices: Some(vec![0, 0, 0, 2, 4, 3, 0, 1, 2]),
            bone_influences_per_vertex: Some(BoneInfluencesPerVertex::Uniform(3)),
            vertex_group_weights: Some(vec![1.0, 0.0, 0.0, 0.5, 0.3, 0.2, 0.6, 0.15, 0.15]),
            ..BlenderMesh::default()
        };

        assert_eq!(three_joints_per_vert, expected_mesh);
    }
}
