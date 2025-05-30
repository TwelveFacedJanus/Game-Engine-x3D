extern crate gl;
extern crate glfw;
extern crate nalgebra_glm as glm;

use glfw::{Action, Context, Key, MouseButton};
use glfw::{GlfwReceiver, fail_on_errors};
use glm::{Mat4, Vec3, vec3};
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::time::Instant;

struct Camera {
    position: Vec3,
    target: Vec3,
    up: Vec3,
    zoom: f32,
    last_mouse_pos: (f64, f64),
    is_rotating: bool,
}

impl Camera {
    fn new() -> Self {
        Camera {
            position: vec3(2.0, 2.0, 2.0),
            target: vec3(0.0, 0.0, 0.0),
            up: vec3(0.0, 1.0, 0.0),
            zoom: 1.0,
            last_mouse_pos: (0.0, 0.0),
            is_rotating: false,
        }
    }

    fn get_view_matrix(&self) -> Mat4 {
        glm::look_at(&(self.position * self.zoom), &self.target, &self.up)
    }

    fn process_mouse(&mut self, window: &glfw::PWindow, xpos: f64, ypos: f64) {
        if self.is_rotating {
            let sensitivity = 0.005;
            let dx = (xpos - self.last_mouse_pos.0) as f32 * sensitivity;
            let dy = (self.last_mouse_pos.1 - ypos) as f32 * sensitivity;

            // Rotate around target
            let right = glm::cross(&(self.position - self.target).normalize(), &self.up);

            // Vertical rotation (pitch)
            let pitch = glm::rotate(&Mat4::identity(), dy, &right);
            let pos_vec4 = glm::vec3_to_vec4(&(self.position - self.target));
            self.position = glm::vec4_to_vec3(&(pitch * pos_vec4)) + self.target;

            // Horizontal rotation (yaw)
            let yaw = glm::rotate(&Mat4::identity(), dx, &self.up);
            let pos_vec4 = glm::vec3_to_vec4(&(self.position - self.target));
            self.position = glm::vec4_to_vec3(&(yaw * pos_vec4)) + self.target;
        }
        self.last_mouse_pos = (xpos, ypos);
    }

    fn process_scroll(&mut self, yoffset: f64) {
        self.zoom -= yoffset as f32 * 0.1;
        self.zoom = self.zoom.max(0.1).min(5.0);
    }
}

pub struct X3D {
    glfw: glfw::Glfw,
    window: glfw::PWindow,
    events: GlfwReceiver<(f64, glfw::WindowEvent)>,
    shader_program: u32,
    vao: u32,
    vbo: u32,
    rotation_angle: f32,
    camera: Camera,
    last_frame_time: Instant,
}

impl X3D {
    pub fn new() -> Self {
        let mut glfw = glfw::init(fail_on_errors!()).unwrap();

        // Window hints for OpenGL
        glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(
            glfw::OpenGlProfileHint::Core,
        ));
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

        let (mut window, events) = glfw
            .create_window(800, 600, "X3D - Camera Control", glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window");

        window.make_current();
        window.set_key_polling(true);
        window.set_mouse_button_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_scroll_polling(true);

        // Initialize OpenGL
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

        // Set up shaders (same as before)
        let shader_program = unsafe {
            let vertex_shader =
                compile_shader(include_str!("shaders/vertex.glsl"), gl::VERTEX_SHADER);
            let fragment_shader =
                compile_shader(include_str!("shaders/fragment.glsl"), gl::FRAGMENT_SHADER);
            link_program(vertex_shader, fragment_shader)
        };

        // Cube data (same as before)
        let vertices = create_cube_vertices();

        // Set up VAO and VBO (same as before)
        let (vao, vbo) = unsafe {
            let mut vao = 0;
            let mut vbo = 0;

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Position attribute
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                6 * mem::size_of::<f32>() as i32,
                ptr::null(),
            );
            gl::EnableVertexAttribArray(0);

            // Normal attribute
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                6 * mem::size_of::<f32>() as i32,
                (3 * mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            (vao, vbo)
        };

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(0.1, 0.1, 0.3, 1.0);
        }

        X3D {
            glfw,
            window,
            events,
            shader_program,
            vao,
            vbo,
            rotation_angle: 0.0,
            camera: Camera::new(),
            last_frame_time: Instant::now(),
        }
    }

    pub fn run(&mut self) {
        while !self.window.should_close() {
            let current_time = Instant::now();
            let delta_time = current_time
                .duration_since(self.last_frame_time)
                .as_secs_f32();
            self.last_frame_time = current_time;

            // Process events
            self.glfw.poll_events();
            for (_, event) in glfw::flush_messages(&self.events) {
                match event {
                    glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true)
                    }
                    glfw::WindowEvent::MouseButton(MouseButton::Button1, Action::Press, _) => {
                        self.camera.is_rotating = true;
                    }
                    glfw::WindowEvent::MouseButton(MouseButton::Button1, Action::Release, _) => {
                        self.camera.is_rotating = false;
                    }
                    glfw::WindowEvent::CursorPos(xpos, ypos) => {
                        self.camera.process_mouse(&self.window, xpos, ypos);
                    }
                    glfw::WindowEvent::Scroll(_, yoffset) => {
                        self.camera.process_scroll(yoffset);
                    }
                    _ => {}
                }
            }

            // Update rotation
            //self.rotation_angle += 0.5 * delta_time;

            // Clear the screen
            unsafe {
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            }

            // Render cube
            self.render_cube();

            // Swap buffers
            self.window.swap_buffers();
        }
    }

    fn render_cube(&self) {
        unsafe {
            gl::UseProgram(self.shader_program);
            gl::BindVertexArray(self.vao);

            // Model matrix (rotation)
            let model = glm::rotate(
                &Mat4::identity(),
                self.rotation_angle,
                &vec3(0.5, 1.0, 0.0).normalize(),
            );

            // View matrix from camera
            let view = self.camera.get_view_matrix();

            // Projection matrix
            let (width, height) = self.window.get_size();
            let projection = glm::perspective(
                width as f32 / height as f32,
                45.0f32.to_radians(),
                0.1,
                100.0,
            );

            // Set matrices
            let model_loc =
                gl::GetUniformLocation(self.shader_program, b"model\0".as_ptr() as *const _);
            let view_loc =
                gl::GetUniformLocation(self.shader_program, b"view\0".as_ptr() as *const _);
            let projection_loc =
                gl::GetUniformLocation(self.shader_program, b"projection\0".as_ptr() as *const _);

            gl::UniformMatrix4fv(model_loc, 1, gl::FALSE, model.as_ptr());
            gl::UniformMatrix4fv(view_loc, 1, gl::FALSE, view.as_ptr());
            gl::UniformMatrix4fv(projection_loc, 1, gl::FALSE, projection.as_ptr());

            // Light position (fixed in world space)
            let light_pos = vec3(1.2, 1.0, 2.0);
            let light_pos_loc =
                gl::GetUniformLocation(self.shader_program, b"lightPos\0".as_ptr() as *const _);
            gl::Uniform3f(light_pos_loc, light_pos.x, light_pos.y, light_pos.z);

            // Draw cube
            gl::DrawArrays(gl::TRIANGLES, 0, 36);
        }
    }
}
unsafe fn compile_shader(src: &str, ty: gl::types::GLenum) -> u32 {
    let shader = gl::CreateShader(ty);
    let c_str = CString::new(src.as_bytes()).unwrap();
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
    gl::CompileShader(shader);

    // Check for compilation errors
    let mut success = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    if success == 0 {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = Vec::with_capacity(len as usize);
        buf.set_len((len as usize) - 1);
        gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut _);
        panic!(
            "Shader compilation failed: {}",
            String::from_utf8_lossy(&buf)
        );
    }

    shader
}

unsafe fn link_program(vertex_shader: u32, fragment_shader: u32) -> u32 {
    let program = gl::CreateProgram();
    gl::AttachShader(program, vertex_shader);
    gl::AttachShader(program, fragment_shader);
    gl::LinkProgram(program);

    // Check for linking errors
    let mut success = 0;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
    if success == 0 {
        let mut len = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = Vec::with_capacity(len as usize);
        buf.set_len((len as usize) - 1);
        gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut _);
        panic!("Program linking failed: {}", String::from_utf8_lossy(&buf));
    }

    gl::DeleteShader(vertex_shader);
    gl::DeleteShader(fragment_shader);

    program
}
fn create_cube_vertices() -> Vec<f32> {
    // Positions + Normals
    vec![
        // Front face
        -0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.5, -0.5, 0.5, 0.0, 0.0, 1.0, 0.5, 0.5, 0.5, 0.0, 0.0, 1.0,
        0.5, 0.5, 0.5, 0.0, 0.0, 1.0, -0.5, 0.5, 0.5, 0.0, 0.0, 1.0, -0.5, -0.5, 0.5, 0.0, 0.0,
        1.0, // Back face
        -0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.5, -0.5, -0.5, 0.0, 0.0, -1.0, 0.5, 0.5, -0.5, 0.0,
        0.0, -1.0, 0.5, 0.5, -0.5, 0.0, 0.0, -1.0, -0.5, 0.5, -0.5, 0.0, 0.0, -1.0, -0.5, -0.5,
        -0.5, 0.0, 0.0, -1.0, // Left face
        -0.5, 0.5, 0.5, -1.0, 0.0, 0.0, -0.5, 0.5, -0.5, -1.0, 0.0, 0.0, -0.5, -0.5, -0.5, -1.0,
        0.0, 0.0, -0.5, -0.5, -0.5, -1.0, 0.0, 0.0, -0.5, -0.5, 0.5, -1.0, 0.0, 0.0, -0.5, 0.5,
        0.5, -1.0, 0.0, 0.0, // Right face
        0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 0.5, 0.5, -0.5, 1.0, 0.0, 0.0, 0.5, -0.5, -0.5, 1.0, 0.0,
        0.0, 0.5, -0.5, -0.5, 1.0, 0.0, 0.0, 0.5, -0.5, 0.5, 1.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.0,
        0.0, 0.0, // Bottom face
        -0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.5, -0.5, -0.5, 0.0, -1.0, 0.0, 0.5, -0.5, 0.5, 0.0,
        -1.0, 0.0, 0.5, -0.5, 0.5, 0.0, -1.0, 0.0, -0.5, -0.5, 0.5, 0.0, -1.0, 0.0, -0.5, -0.5,
        -0.5, 0.0, -1.0, 0.0, // Top face
        -0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.5, 0.5, -0.5, 0.0, 1.0, 0.0, 0.5, 0.5, 0.5, 0.0, 1.0,
        0.0, 0.5, 0.5, 0.5, 0.0, 1.0, 0.0, -0.5, 0.5, 0.5, 0.0, 1.0, 0.0, -0.5, 0.5, -0.5, 0.0,
        1.0, 0.0,
    ]
}

fn main() {
    let mut x3d = X3D::new();
    x3d.run();
}
