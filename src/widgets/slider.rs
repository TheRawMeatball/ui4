use std::sync::Arc;

use bevy::{
    ecs::{prelude::*, system::SystemState},
    math::*,
    prelude::GlobalTransform,
    window::Windows,
};

use crate::dom::Node;

#[derive(Component)]
pub struct EngagedSlider {
    pub(super) initial_offset: Vec2,
    pub(super) slider_entity: Entity,
    pub(super) get_percent: Arc<dyn Fn(&mut World) -> &mut f32 + Send + Sync>,
}

pub(crate) struct SliderSystemState {
    state: SystemState<(
        Query<'static, 'static, &'static EngagedSlider>,
        Query<'static, 'static, (&'static Node, &'static GlobalTransform)>,
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
        let (engaged, slider, windows) = self.state.get(world);
        let cursor_pos = windows
            .get_primary()
            .and_then(|window| window.cursor_position());
        if let (Ok(engaged), Some(cursor_pos)) = (engaged.get_single(), cursor_pos) {
            let (node, pos) = slider.get(engaged.slider_entity).unwrap();
            let len = node.size.x;
            let start = pos.translation.x - len / 2.;
            let current = cursor_pos.x - engaged.initial_offset.x;
            let percent = (current - start) / len;
            let gp = engaged.get_percent.clone();
            let p = gp(world);
            *p = percent;
        }
    }

    pub(crate) fn system(world: &mut World) {
        world.resource_scope(|w, mut x: Mut<Self>| {
            x.run(w);
        })
    }
}
