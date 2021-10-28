use bevy::{
    ecs::prelude::*,
    math::Vec2,
    prelude::{GlobalTransform, Transform},
};

pub mod layout;
pub mod render;

#[derive(Component, Default)]
pub(crate) struct Control;

#[derive(Bundle, Default)]
pub(crate) struct ControlBundle {
    control: Control,
    transform: Transform,
    global_transform: GlobalTransform,
}

#[derive(Component, Default)]
pub(crate) struct Node {
    pub pos: Vec2,
    pub size: Vec2,
}

#[derive(Bundle, Default)]
pub(crate) struct NodeBundle {
    node: Node,
    transform: Transform,
    global_transform: GlobalTransform,
}

#[derive(Component)]
pub struct Text(pub String);

#[derive(Component)]
pub struct TextFont(epaint::TextStyle);

#[derive(Component)]
pub struct Color(pub bevy::prelude::Color);

impl Color {
    fn as_rgba_u8(&self) -> [u8; 4] {
        let [r, g, b, a] = self.0.as_rgba_f32();
        [
            (r * u8::MAX as f32) as u8,
            (g * u8::MAX as f32) as u8,
            (b * u8::MAX as f32) as u8,
            (a * u8::MAX as f32) as u8,
        ]
    }
}

#[derive(Component)]
pub enum Interaction {
    Clicked,
    Hovered,
    None,
}
