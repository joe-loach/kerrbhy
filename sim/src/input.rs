use std::collections::HashMap;

use event::Event;
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
};

pub struct Mouse {
    x: f32,
    y: f32,
    button_states: HashMap<MouseButton, bool>,
}

impl Default for Mouse {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            button_states: Default::default(),
        }
    }
}

impl Mouse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_state(&mut self, event: &Event<()>) {
        if let Event::Window(e) = event {
            match e {
                WindowEvent::CursorMoved { position, .. } => {
                    self.x = position.x as f32;
                    self.y = position.y as f32;
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let down = state.is_pressed();
                    self.button_states
                        .entry(*button)
                        .and_modify(|e| *e = down)
                        .or_insert(down);
                }
                _ => (),
            }
        }
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
