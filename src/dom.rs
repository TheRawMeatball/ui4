use bevy::{
    ecs::prelude::*,
    math::Vec2,
    prelude::{GlobalTransform, Transform},
};
use bevy_inspector_egui::Inspectable;

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

#[derive(Component, Default, Inspectable)]
pub(crate) struct Node {
    pub pos: Vec2,
    pub size: Vec2,
}

#[derive(Component, Default, Inspectable)]
pub(crate) struct ClippedNode {
    pub min: Vec2,
    pub max: Vec2,
    pub z_layer: u32,
}

#[derive(Bundle, Default)]
pub(crate) struct NodeBundle {
    node: Node,
    clipped: ClippedNode,
}

#[derive(Component, Inspectable)]
pub struct Text(pub String);

#[derive(Component, Inspectable)]
pub struct TextSize(pub f32);

#[derive(Component)]
pub struct TextFont(pub epaint::TextStyle);

#[derive(Component)]
pub struct ShowOverflow;

/// Overrides [`TextFont`] and [`TextFont`]
#[derive(Component)]
pub struct TextDetails(pub Vec<epaint::text::LayoutSection>);

#[derive(Component, Inspectable)]
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

#[derive(Component, PartialEq, Eq, Inspectable, Debug)]
pub enum Interaction {
    Clicked,
    Hovered,
    None,
}

#[derive(Component, Clone, Inspectable)]
pub enum FocusPolicy {
    Block,
    Pass,
}

#[derive(Component)]
pub struct Focused(pub(crate) ());
#[derive(Component)]
pub struct Focusable;
