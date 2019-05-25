use crate::state_wrapper::msg::Msg;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use web_sys::window;

pub struct State {
    pub last_tick_time: SystemTime,
    pub app_start_time: SystemTime,
    /// The model that the user is currently viewing in their browser
    pub current_model: String,
    camera_distance: f32,
}

impl State {
    pub fn new() -> State {
        State {
            last_tick_time: State::performance_now_to_system_time(),
            app_start_time: State::performance_now_to_system_time(),
            current_model: "TexturedCube".to_string(),
            camera_distance: 10.
            //            current_model: "TriangulatedCube".to_string(),
        }
    }

    pub fn performance_now_to_system_time() -> SystemTime {
        let now = window().unwrap().performance().unwrap().now();

        let seconds = (now as u64) / 1_000;
        let nanos = ((now as u32) % 1_000) * 1_000_000;

        UNIX_EPOCH + Duration::new(seconds, nanos)
    }
}

impl State {
    pub fn msg(&mut self, msg: Msg) {
        match msg {
            Msg::Zoom(zoom) => {
                self.camera_distance += zoom;
            }
            Msg::SetCurrentMesh(mesh_name) => self.current_model = mesh_name.to_string(),
        }
    }
}

impl State {
    pub fn camera_distance(&self) -> f32 {
        self.camera_distance
    }
}
