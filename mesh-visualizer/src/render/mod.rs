use assets::Assets;
use blender_mesh::BlenderMesh;
use cgmath;
use cgmath::Matrix4;
use cgmath::Point3;
use cgmath::Vector3;
use shader::Shader;
use shader::ShaderSystem;
use shader::ShaderType;
use std::f32::consts::PI;
use std::rc::Rc;
use web_apis::log;
use web_apis::WebGLBuffer;
use web_apis::WebGLProgram;
use web_apis::WebGLRenderingContext;
use State;

// Temporarily using u16's until I can get GLbitfield / Glenum etc working
static gl_COLOR_BUFFER_BIT: u16 = 16384;
static gl_DEPTH_BUFFER_BIT: u16 = 256;
// color_buffer_bit | depth_buffer_bit
static BITFIELD: u16 = 16640;

static gl_ARRAY_BUFFER: u16 = 34962;
static gl_ELEMENT_ARRAY_BUFFER: u16 = 34963;
static gl_FLOAT: u16 = 5126;
static gl_STATIC_DRAW: u16 = 35044;

pub struct Renderer {
    gl: Rc<WebGLRenderingContext>,
    assets: Assets,
    shader_sys: ShaderSystem,
    state: Rc<State>,
}

trait Render {
    fn shader_type(&self) -> ShaderType;
    fn render(&self, gl: &WebGLRenderingContext, shader_program: &Shader);
    fn buffer_f32_data(
        &self,
        gl: &WebGLRenderingContext,
        buf: &WebGLBuffer,
        // TODO: &Vec<f32>
        data: Vec<f32>,
        attrib_loc: u16,
        size: u8,
    ) {
        gl.bind_buffer(gl_ARRAY_BUFFER, &buf);
        gl.buffer_f32_data(gl_ARRAY_BUFFER, data, gl_STATIC_DRAW);
        gl.vertex_attrib_pointer(attrib_loc, size, gl_FLOAT, false, 0, 0);
    }
    // TODO: Generics
    fn buffer_u8_data(
        &self,
        gl: &WebGLRenderingContext,
        buf: &WebGLBuffer,
        // TODO: &Vec<f32>
        data: Vec<u8>,
        attrib_loc: u16,
        size: u8,
    ) {
        gl.bind_buffer(gl_ARRAY_BUFFER, &buf);
        gl.buffer_u8_data(gl_ARRAY_BUFFER, data, gl_STATIC_DRAW);
        gl.vertex_attrib_pointer(attrib_loc, size, gl_FLOAT, false, 0, 0);
    }
}
trait BlenderMeshRender {
    fn render_non_skinned(&self, gl: &WebGLRenderingContext, shader_program: &Shader);
    fn render_dual_quat_skinned(&self, gl: &WebGLRenderingContext, shader_program: &Shader);
}

struct attribute<T>(T);
struct uniform<T>(T);

// TODO: These types can probably be automatically generated based on the shader
struct NonSkinnedRender {
    aVertexPos: attribute<Vec<f32>>,
    aVertexNormal: attribute<Vec<f32>>,
    uMVMatrix: uniform<Vec<f32>>,
}

impl Render for BlenderMesh {
    fn shader_type(&self) -> ShaderType {
        if let Some(_) = self.armature_name {
            ShaderType::DualQuatSkin
        } else {
            ShaderType::NonSkinned
        }
    }
    fn render(&self, gl: &WebGLRenderingContext, shader: &Shader) {
        if let Some(_) = self.armature_name {
            self.render_dual_quat_skinned(&gl, &shader);
        } else {
            self.render_non_skinned(&gl, &shader);
        }
    }
}

impl BlenderMeshRender for BlenderMesh {
    fn render_non_skinned(&self, gl: &WebGLRenderingContext, shader: &Shader) {
        let vertex_pos_attrib = gl.get_attrib_location(&shader.program, "aVertexPosition");
        gl.enable_vertex_attrib_array(vertex_pos_attrib);

        let vertex_normal_attrib = gl.get_attrib_location(&shader.program, "aVertexNormal");
        gl.enable_vertex_attrib_array(vertex_normal_attrib);

        gl.clear(BITFIELD);

        let fovy = cgmath::Rad(PI / 3.0);
        let perspective = cgmath::perspective(fovy, 1.0, 0.1, 100.0);
        let mut p_matrix = vec_from_matrix4(&perspective);

        let model_matrix = Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0));

        let mut mv_matrix = Matrix4::look_at(
            Point3::new(1.0, 2.0, 2.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        );

        // TODO: Breadcrumb - add normal and point lighting to shader..

        // TODO: Multiply without new allocation
        mv_matrix = mv_matrix * model_matrix;

        let mv_matrix = vec_from_matrix4(&mv_matrix);

        let p_matrix_uni = gl.get_uniform_location(&shader.program, "uPMatrix");
        let mv_matrix_uni = gl.get_uniform_location(&shader.program, "uMVMatrix");

        gl.uniform_matrix_4fv(p_matrix_uni, false, p_matrix);
        gl.uniform_matrix_4fv(mv_matrix_uni, false, mv_matrix);

        let pos = self.vertex_positions.clone();
        self.buffer_f32_data(&gl, &shader.buffers[0], pos, vertex_pos_attrib, 3);

        let norms = self.vertex_normals.clone();
        self.buffer_f32_data(&gl, &shader.buffers[1], norms, vertex_normal_attrib, 3);

        let index_buffer = gl.create_buffer();
        gl.bind_buffer(gl_ELEMENT_ARRAY_BUFFER, &index_buffer);

        let ind = self.vertex_position_indices.clone();
        gl.buffer_u16_data(gl_ELEMENT_ARRAY_BUFFER, ind, gl_STATIC_DRAW);

        let gl_TRIANGLES = 4;
        let gl_UNSIGNED_SHORT = 5123;

        gl.bind_buffer(gl_ELEMENT_ARRAY_BUFFER, &index_buffer);

        gl.draw_elements(
            gl_TRIANGLES,
            self.vertex_position_indices.len() as u16,
            gl_UNSIGNED_SHORT,
            0,
        );
    }

    fn render_dual_quat_skinned(&self, gl: &WebGLRenderingContext, shader: &Shader) {
        let vertex_pos_attrib = gl.get_attrib_location(&shader.program, "aVertexPosition");
        gl.enable_vertex_attrib_array(vertex_pos_attrib);

        let vertex_normal_attrib = gl.get_attrib_location(&shader.program, "aVertexNormal");
        gl.enable_vertex_attrib_array(vertex_normal_attrib);

        let joint_index_attrib = gl.get_attrib_location(&shader.program, "aJointIndex");
        gl.enable_vertex_attrib_array(joint_index_attrib);

        let joint_weight_attrib = gl.get_attrib_location(&shader.program, "aJointWeight");
        gl.enable_vertex_attrib_array(joint_weight_attrib);

        gl.clear(BITFIELD);

        let fovy = cgmath::Rad(PI / 3.0);
        let perspective = cgmath::perspective(fovy, 1.0, 0.1, 100.0);
        let mut p_matrix = vec_from_matrix4(&perspective);

        let model_matrix = Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0));

        let mut mv_matrix = Matrix4::look_at(
            Point3::new(1.0, 2.0, 2.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        );

        // TODO: Breadcrumb - add normal and point lighting to shader..

        // TODO: Multiply without new allocation
        mv_matrix = mv_matrix * model_matrix;

        let mv_matrix = vec_from_matrix4(&mv_matrix);

        let p_matrix_uni = gl.get_uniform_location(&shader.program, "uPMatrix");
        gl.uniform_matrix_4fv(p_matrix_uni, false, p_matrix);

        let mv_matrix_uni = gl.get_uniform_location(&shader.program, "uMVMatrix");
        gl.uniform_matrix_4fv(mv_matrix_uni, false, mv_matrix);

        let pos = self.vertex_positions.clone();
        self.buffer_f32_data(&gl, &shader.buffers[0], pos, vertex_pos_attrib, 3);

        let norms = self.vertex_normals.clone();
        self.buffer_f32_data(&gl, &shader.buffers[1], norms, vertex_normal_attrib, 3);

        let joints = self.vertex_group_indices.as_ref().unwrap().clone();
        self.buffer_u8_data(&gl, &shader.buffers[2], joints, joint_index_attrib, 4);

        let weights = self.vertex_group_indices.as_ref().unwrap().clone();
        self.buffer_u8_data(&gl, &shader.buffers[3], weights, joint_weight_attrib, 4);

        let index_buffer = gl.create_buffer();
        gl.bind_buffer(gl_ELEMENT_ARRAY_BUFFER, &index_buffer);

        // TODO: Remove clone
        gl.buffer_u16_data(
            gl_ELEMENT_ARRAY_BUFFER,
            self.vertex_position_indices.clone(),
            gl_STATIC_DRAW,
        );

        let gl_TRIANGLES = 4;
        let gl_UNSIGNED_SHORT = 5123;

        gl.bind_buffer(gl_ELEMENT_ARRAY_BUFFER, &index_buffer);

        gl.draw_elements(
            gl_TRIANGLES,
            self.vertex_position_indices.len() as u16,
            gl_UNSIGNED_SHORT,
            0,
        );
    }
}

impl Renderer {
    pub fn new(
        gl: Rc<WebGLRenderingContext>,
        assets: Assets,
        shader_sys: ShaderSystem,
        state: Rc<State>,
    ) -> Renderer {
        Renderer {
            gl,
            assets,
            shader_sys,
            state,
        }
    }

    pub fn render(&self) {
        let mesh = self.assets.meshes();
        let mesh = mesh.borrow();
        // let mesh = mesh.get(&state.current_model);
        let mesh = mesh.get("LetterF");

        if mesh.is_none() {
            return;
        }

        let mesh = mesh.unwrap();

        self.shader_sys.use_program(&mesh.shader_type());

        // TODO: Breadcrumb - armature.buffer_data() to buffer the bone quaternions into the GPU

        mesh.render(&self.gl, self.shader_sys.get_shader(&mesh.shader_type()));
    }
}

fn vec_from_matrix4(mat4: &Matrix4<f32>) -> Vec<f32> {
    // TODO: Accept output vec instead of re-allocating
    let mut vec = vec![];

    for index in 0..16 {
        vec.push(mat4[index / 4][index % 4]);
    }

    vec
}
