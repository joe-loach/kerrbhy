use std::collections::HashMap;

use event::Event;
use glam::{
    vec2,
    Vec2,
};
use winit::{
    event::{
        MouseButton,
        WindowEvent,
    },
    keyboard::{
        KeyCode,
        ModifiersState,
        PhysicalKey,
    },
    window::Window,
};

pub struct Mouse {
    pos: Vec2,
    scroll_delta: Vec2,
    button_states: HashMap<MouseButton, bool>,
}

impl Default for Mouse {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
            button_states: Default::default(),
        }
    }
}

impl Mouse {
    pub const PIXELS_PER_LINE: f32 = 50.0;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_state(&mut self, window: &Window, event: &Event<()>) {
        if let Event::Window(e) = event {
            match e {
                WindowEvent::CursorMoved { position, .. } => {
                    self.pos.x = position.x as f32;
                    self.pos.y = position.y as f32;
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let down = state.is_pressed();
                    self.button_states
                        .entry(*button)
                        .and_modify(|e| *e = down)
                        .or_insert(down);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    self.scroll_delta = match *delta {
                        winit::event::MouseScrollDelta::LineDelta(x, y) => {
                            vec2(x, y) * Self::PIXELS_PER_LINE
                        }
                        winit::event::MouseScrollDelta::PixelDelta(delta) => {
                            vec2(delta.x as f32, delta.y as f32) / window.scale_factor() as f32
                        }
                    }
                }
                _ => (),
            }
        }
    }

    pub fn smooth(&mut self, dt: f32) {
        const DECAY_RATE: f32 = 5.0;
        // moving at 1/4th of a pixel
        const CLOSE_TO_ZERO: f32 = 0.25;

        let decay = (-DECAY_RATE * dt).exp();
        let smoothed = self.scroll_delta * decay;

        // stop moving if the velocity is close to zero
        if smoothed.abs_diff_eq(Vec2::ZERO, CLOSE_TO_ZERO) {
            self.scroll_delta = Vec2::ZERO
        } else {
            self.scroll_delta = smoothed;
        }
    }

    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    pub fn clicked(&self, button: MouseButton) -> bool {
        self.button_states
            .get(&MouseButton::Left)
            .is_some_and(|&down| down)
    }

    pub fn left_clicked(&self) -> bool {
        self.clicked(MouseButton::Left)
    }

    pub fn right_clicked(&self) -> bool {
        self.clicked(MouseButton::Right)
    }
}

#[derive(Default)]
pub struct Keyboard {
    key_states: HashMap<KeyCode, bool>,
    modifiers: ModifiersState,
}

impl Keyboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_state(&mut self, event: &Event<()>) {
        if let Event::Window(e) = event {
            match e {
                WindowEvent::KeyboardInput { event, .. } => {
                    if let PhysicalKey::Code(key) = event.physical_key {
                        let down = event.state.is_pressed();
                        self.key_states
                            .entry(key)
                            .and_modify(|e| *e = down)
                            .or_insert(down);
                    }
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    self.modifiers = modifiers.state();
                }
                _ => (),
            }
        }
    }

    pub fn is_down(&self, key: KeyCode) -> bool {
        self.key_states.get(&key).is_some_and(|&down| down)
    }
}
