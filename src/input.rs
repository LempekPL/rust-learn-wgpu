use std::collections::HashSet;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct Keyboard {
    current_pressed: HashSet<KeyCode>,
    last_pressed: HashSet<KeyCode>,
}

#[allow(unused)]
impl Keyboard {
    pub fn new() -> Self {
        Self {
            current_pressed: HashSet::new(),
            last_pressed: HashSet::new(),
        }
    }

    pub fn update(&mut self) {
        self.last_pressed = self.current_pressed.clone();
    }

    pub fn handle_event(&mut self, event: &KeyEvent) {
        if let PhysicalKey::Code(code) = event.physical_key {
            match event.state {
                ElementState::Pressed => {
                    if !event.repeat {
                        self.current_pressed.insert(code);
                    }
                }
                ElementState::Released => {
                    self.current_pressed.remove(&code);
                }
            }
        }
    }

    pub fn is_pressed(&self, code: KeyCode) -> bool {
        self.current_pressed.contains(&code)
    }

    pub fn is_released(&self, code: KeyCode) -> bool {
        !self.current_pressed.contains(&code)
    }

    pub fn just_pressed(&self, code: KeyCode) -> bool {
        self.current_pressed.contains(&code) && !self.last_pressed.contains(&code)
    }

    pub fn just_released(&self, code: KeyCode) -> bool {
        !self.current_pressed.contains(&code) && self.last_pressed.contains(&code)
    }
}

pub struct Mouse {
    position: (f64, f64),
    delta: (f64, f64),
    scroll_delta: (f64, f64),

    current_pressed: HashSet<MouseButton>,
    last_pressed: HashSet<MouseButton>,
}

#[allow(unused)]
impl Mouse {
    pub fn new() -> Self {
        Self {
            position: (0.0, 0.0),
            delta: (0.0, 0.0),
            scroll_delta: (0.0, 0.0),
            current_pressed: HashSet::new(),
            last_pressed: HashSet::new(),
        }
    }

    pub fn update(&mut self) {
        self.last_pressed = self.current_pressed.clone();
        self.delta = (0.0, 0.0);
    }

    pub fn handle_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let new_pos = (position.x, position.y);
        self.delta = (new_pos.0 - self.position.0, new_pos.1 - self.position.1);
        self.position = new_pos;
    }

    pub fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        match state {
            ElementState::Pressed => {
                self.current_pressed.insert(button);
            }
            ElementState::Released => {
                self.current_pressed.remove(&button);
            }
        }
    }

    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                self.scroll_delta = (x as f64, y as f64);
            }
            MouseScrollDelta::PixelDelta(pos) => {
                self.scroll_delta = (pos.x, pos.y);
            }
        }
    }

    pub fn position(&self) -> (f64, f64) {
        self.position
    }

    pub fn delta(&self) -> (f64, f64) {
        self.delta
    }

    pub fn is_pressed(&self, button: MouseButton) -> bool {
        self.current_pressed.contains(&button)
    }

    pub fn is_released(&self, button: MouseButton) -> bool {
        !self.current_pressed.contains(&button)
    }

    pub fn just_pressed(&self, button: MouseButton) -> bool {
        self.current_pressed.contains(&button) && !self.last_pressed.contains(&button)
    }

    pub fn just_released(&self, button: MouseButton) -> bool {
        !self.current_pressed.contains(&button) && self.last_pressed.contains(&button)
    }
}

