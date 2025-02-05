use crate::assets::Assets;
use crate::shader::Shader;
use crate::shader::ShaderKind;
use crate::shader::ShaderKind::NonSkinnedNonTextured;
use crate::shader::ShaderSystem;
use crate::state_wrapper::State;
use blender_armature::BlenderArmature;
use blender_mesh::{BlenderMesh, MaterialInput};
use js_sys::WebAssembly;
use nalgebra::Perspective3;
use nalgebra::{Isometry3, Point3, Vector3};
use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::WebGlRenderingContext as GL;
use web_sys::*;

mod armature_render;
mod mesh_render;

pub struct Renderer {
    gl: Rc<WebGlRenderingContext>,
    assets: Rc<RefCell<Assets>>,
    shader_sys: Rc<ShaderSystem>,
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum VaoKey {
    // TODO: Instead of String, an enum that's auto generated from mesh names in the Blender
    // files via build.rs
    MeshName(String),
}

pub enum RenderInstructions {
    DrawElements { num_indices: i32 },
}

pub trait Renderable {
    fn shader_kind(&self) -> ShaderKind;

    fn vao_key(&self) -> VaoKey;

    fn buffer_attributes(&self, gl: &WebGlRenderingContext, shader: &Shader);

    fn set_uniforms(
        &self,
        gl: &WebGlRenderingContext,
        shader: &Shader,
        state: &State,
    ) -> RenderInstructions;
}

trait BlenderMeshRender {
    fn render_non_skinned(&self, gl: &WebGlRenderingContext, shader_program: &Shader);
    fn render_dual_quat_skinned(&self, gl: &WebGlRenderingContext, shader_program: &Shader);
}

struct Attrubute<T>(T);
struct Uniform<T>(T);

// TODO: These types can probably be automatically generated based on the shader
struct NonSkinnedMesh<'a> {
    blender_mesh: &'a BlenderMesh,
    name: String,
}

impl<'a> Renderable for NonSkinnedMesh<'a> {
    fn shader_kind(&self) -> ShaderKind {
        //        if let Some(_) = self.armature_name {
        //            ShaderKind::DualQuatSkin
        //        } else {
        //            ShaderKind::NonSkinned
        //        }
        if let Some(_) = self.blender_mesh.vertex_uvs {
            ShaderKind::NonSkinnedWithTexture
        } else {
            ShaderKind::NonSkinnedNonTextured
        }
    }

    fn vao_key(&self) -> VaoKey {
        VaoKey::MeshName(self.name.clone())
    }

    fn buffer_attributes(&self, gl: &WebGlRenderingContext, shader: &Shader) {
        let pos_attrib =
            gl.get_attrib_location(&shader.program.as_ref().unwrap(), "aVertexPosition");
        let normal_attrib =
            gl.get_attrib_location(&shader.program.as_ref().unwrap(), "aVertexNormal");

        gl.enable_vertex_attrib_array(pos_attrib as u32);
        gl.enable_vertex_attrib_array(normal_attrib as u32);

        if let Some(ref uvs) = self.blender_mesh.vertex_uvs.as_ref() {
            let uv_attrib =
                gl.get_attrib_location(shader.program.as_ref().unwrap(), "aTextureCoord");
            gl.enable_vertex_attrib_array(uv_attrib as u32);

            GpuBufferer::buffer_f32_data(&gl, &uvs[..], uv_attrib as u32, 2);
        }

        let mesh = self.blender_mesh;

        GpuBufferer::buffer_f32_data(&gl, &mesh.vertex_positions[..], pos_attrib as u32, 3);
        GpuBufferer::buffer_f32_data(&gl, &mesh.vertex_normals[..], normal_attrib as u32, 3);

        GpuBufferer::buffer_u16_indices(&gl, &mesh.vertex_position_indices[..]);
    }

    fn set_uniforms(
        &self,
        gl: &WebGlRenderingContext,
        shader: &Shader,
        state: &State,
    ) -> RenderInstructions {
        // TODO: Cache uniform locations in the Shader.
        let perspective_uni =
            gl.get_uniform_location(shader.program.as_ref().unwrap(), "perspective");
        let perspective = state.camera().projection();
        gl.uniform_matrix4fv_with_f32_array(perspective_uni.as_ref(), false, &perspective);

        let view = state.camera().view();

        let view_uni = gl.get_uniform_location(shader.program.as_ref().unwrap(), "view");
        let view_uni = view_uni.as_ref();

        let pos = (0.0, 0.0, 0.0);
        let model = Isometry3::new(Vector3::new(pos.0, pos.1, pos.2), nalgebra::zero());
        let model = model.to_homogeneous();

        let mut model_array = [0.; 16];

        model_array.copy_from_slice(model.as_slice());

        let model_uni = gl.get_uniform_location(shader.program.as_ref().unwrap(), "model");
        let model_uni = model_uni.as_ref();

        gl.uniform_matrix4fv_with_f32_array(model_uni, false, &mut model_array);
        gl.uniform_matrix4fv_with_f32_array(view_uni, false, &view);

        // FIXME: Add materials to both shaders .. not just non textured ..
        if self.shader_kind() == NonSkinnedNonTextured {
            let base_color_uni =
                gl.get_uniform_location(shader.program.as_ref().unwrap(), "baseColor");

            let base_color = match self.blender_mesh.materials().iter().next() {
                Some((_, material)) => match material.base_color() {
                    MaterialInput::Uniform(color) => color,
                    _ => &[0.8, 0.552, 0.017],
                },
                None => &[0.8, 0.552, 0.017],
            };

            gl.uniform3fv_with_f32_array(base_color_uni.as_ref(), base_color);
        }

        let camera_pos_uni =
            gl.get_uniform_location(shader.program.as_ref().unwrap(), "uCameraPos");
        let camera_pos = state.camera().get_eye_pos();
        gl.uniform3fv_with_f32_array(
            camera_pos_uni.as_ref(),
            &[camera_pos[0], camera_pos[1], camera_pos[2]],
        );

        let roughness_uni = gl.get_uniform_location(shader.program.as_ref().unwrap(), "roughness");
        gl.uniform1f(roughness_uni.as_ref(), state.roughness());

        let metallic_uni = gl.get_uniform_location(shader.program.as_ref().unwrap(), "metallic");
        gl.uniform1f(metallic_uni.as_ref(), state.metallic());

        let light_pos_uni = gl.get_uniform_location(shader.program.as_ref().unwrap(), "lightPos");
        gl.uniform3fv_with_f32_array(light_pos_uni.as_ref(), &[1.1, 1.1, 1.1]);

        let light_color_uni =
            gl.get_uniform_location(shader.program.as_ref().unwrap(), "lightColor");
        gl.uniform3fv_with_f32_array(light_color_uni.as_ref(), &[1.0, 1.0, 1.0]);

        let num_indices = self.blender_mesh.vertex_position_indices.len() as i32;
        RenderInstructions::DrawElements { num_indices }
    }
}

impl Renderer {
    pub fn new(
        gl: Rc<WebGlRenderingContext>,
        assets: Rc<RefCell<Assets>>,
        shader_sys: Rc<ShaderSystem>,
    ) -> Renderer {
        Renderer {
            gl,
            assets,
            shader_sys,
        }
    }

    pub fn render(&self, state: &State) {
        let gl = &self.gl;

        gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);

        let mesh = self.assets.borrow().meshes();
        let mesh = mesh.borrow();
        let mesh = mesh.get(state.current_model.as_str());

        if mesh.is_none() {
            return;
        }

        let mesh = mesh.unwrap();

        let renderable_mesh = NonSkinnedMesh {
            blender_mesh: &mesh,
            name: "Foo".to_string(),
        };

        self.shader_sys.use_program(&renderable_mesh.shader_kind());

        let shader = self.shader_sys.get_shader(&renderable_mesh.shader_kind());

        if shader.is_none() {
            return;
        }

        let shader = shader.unwrap();

        //        if mesh.armature_name.is_some() {
        //            let armature = self.assets.borrow().armatures();
        //            let armature = armature.borrow();
        //            let armature = armature.get(mesh.armature_name.as_ref().unwrap());
        //
        //            if armature.is_none() {
        //                return;
        //            }
        //
        //            armature.unwrap().buffer_data(&self.gl, shader, &state);
        //        }

        // TODO: Use VAOs and only buffer attributes once.
        renderable_mesh.buffer_attributes(&self.gl, shader);

        match renderable_mesh.set_uniforms(&self.gl, shader, state) {
            RenderInstructions::DrawElements { num_indices } => {
                gl.draw_elements_with_i32(GL::TRIANGLES, num_indices, GL::UNSIGNED_SHORT, 0);
            }
        }
    }
}

pub struct GpuBufferer;

impl GpuBufferer {
    pub fn buffer_f32_data(gl: &GL, data: &[f32], attrib: u32, size: i32) {
        let memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()
            .unwrap()
            .buffer();

        let data_location = data.as_ptr() as u32 / 4;

        let data_array = js_sys::Float32Array::new(&memory_buffer)
            .subarray(data_location, data_location + data.len() as u32);

        let buffer = gl.create_buffer().unwrap();

        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &data_array, GL::STATIC_DRAW);
        gl.vertex_attrib_pointer_with_i32(attrib, size, GL::FLOAT, false, 0, 0);
    }

    pub fn buffer_u8_data(gl: &GL, data: &[u8], attrib: u32, size: i32) {
        let memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()
            .unwrap()
            .buffer();

        let data_location = data.as_ptr() as u32;

        let data_array = js_sys::Uint8Array::new(&memory_buffer)
            .subarray(data_location, data_location + data.len() as u32);

        let buffer = gl.create_buffer().unwrap();

        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));
        gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &data_array, GL::STATIC_DRAW);
        gl.vertex_attrib_pointer_with_i32(attrib, size, GL::UNSIGNED_BYTE, false, 0, 0);
    }

    pub fn buffer_u16_indices(gl: &GL, indices: &[u16]) {
        let memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()
            .unwrap()
            .buffer();

        let indices_location = indices.as_ptr() as u32 / 2;
        let indices_array = js_sys::Uint16Array::new(&memory_buffer)
            .subarray(indices_location, indices_location + indices.len() as u32);

        let index_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));
        gl.buffer_data_with_array_buffer_view(
            GL::ELEMENT_ARRAY_BUFFER,
            &indices_array,
            GL::STATIC_DRAW,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fo() {}
}
