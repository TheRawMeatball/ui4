use bevy::{
    ecs::prelude::*,
    math::Vec2,
    prelude::{GlobalTransform, Transform},
};

pub mod layout;

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
    pos: Vec2,
    size: Vec2,
}

#[derive(Bundle, Default)]
pub(crate) struct NodeBundle {
    node: Node,
    transform: Transform,
    global_transform: GlobalTransform,
}
