use bevy::{
    ecs::prelude::*,
    math::Vec2,
    text::{TextAlignment, TextStyle},
};
use bevy_inspector_egui::Inspectable;

pub mod layout;
pub mod render;

#[derive(Component, Default)]
pub(crate) struct Control;

#[derive(Bundle, Default)]
pub(crate) struct ControlBundle {
    control: Control,
}

#[derive(Component, Default, Inspectable, Clone, Copy)]
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
pub struct UiText(pub String);

#[derive(Component, Inspectable)]
pub struct TextSize(pub f32);

#[derive(Component, Inspectable)]
pub struct TextBoxCursor(pub Option<usize>);

#[derive(Component, Inspectable)]
pub struct TextDetails(pub Vec<(TextStyle, usize)>);

#[derive(Component, Inspectable)]
pub struct TextAlign(pub TextAlignment);

#[derive(Component)]
pub struct HideOverflow;

#[derive(Component, PartialEq, Eq, Debug, Clone, Copy, Inspectable)]
pub enum Interaction {
    Clicked,
    Hovered,
    None,
}

impl Default for Interaction {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Component, Clone)]
pub enum FocusPolicy {
    Block,
    Pass,
}

#[derive(Component)]
pub struct Focused(pub(crate) ());
#[derive(Component)]
pub struct Focusable;
