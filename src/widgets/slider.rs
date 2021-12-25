use std::sync::Arc;

use bevy::{
    ecs::{prelude::*, system::SystemState},
    math::*,
    window::Windows,
};

#[derive(Component)]
pub struct EngagedSlider {
    pub(super) process: Arc<dyn Fn(&mut World, Vec2) + Send + Sync>,
}

pub(crate) struct SliderSystemState {
    state: SystemState<(
        Query<'static, 'static, &'static EngagedSlider>,
        Res<'static, Windows>,
    )>,
}

impl FromWorld for SliderSystemState {
    fn from_world(world: &mut World) -> Self {
        Self {
            state: SystemState::new(world),
        }
    }
}

impl SliderSystemState {
    fn run(&mut self, world: &mut World) {
        let (engaged, windows) = self.state.get(world);
        let cursor_pos = windows
            .get_primary()
            .and_then(|window| window.cursor_position());
        if let (Ok(engaged), Some(cursor_pos)) = (engaged.get_single(), cursor_pos) {
            let gp = engaged.process.clone();
            gp(world, cursor_pos);
        }
    }

    pub(crate) fn system(world: &mut World) {
        world.resource_scope(|w, mut x: Mut<Self>| {
            x.run(w);
        })
    }
}
