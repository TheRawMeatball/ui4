use bevy::ecs::prelude::*;
use bevy::math::*;

mod layout;

#[derive(Component)]
struct FinalSize(Vec2);

#[derive(Component)]
pub struct Control;
