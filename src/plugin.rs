use bevy::app::Plugin;
use bevy::ecs::prelude::*;
use bevy::prelude::CoreStage;

use crate::animation::RunningTweens;
use crate::button::ButtonSystemState;
use crate::ctx::Ctx;
use crate::runtime::{primary_ui_system, UiManagedSystems, UiScratchSpace};
use crate::textbox::TextBoxSystemState;

pub struct Ui4Plugin;
impl Plugin for Ui4Plugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<UiScratchSpace>()
            .init_resource::<ButtonSystemState>()
            .init_resource::<TextBoxSystemState>()
            .init_resource::<RunningTweens>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(primary_ui_system.exclusive_system().at_end())
            .add_system(crate::textbox::focus_system)
            .add_system(crate::animation::tween_system)
            .add_system_to_stage(CoreStage::PostUpdate, crate::animation::transition_system);
    }
}

pub struct Ui4Root<F>(pub F);
impl<F: Fn(Ctx) -> Ctx + Clone + Send + Sync + 'static> Plugin for Ui4Root<F> {
    fn build(&self, app: &mut bevy::prelude::App) {
        let root = self.0.clone();
        app.add_startup_system(
            (move |world: &mut World| init_ui(world, &root))
                .exclusive_system()
                .at_end(),
        );
    }
}

fn init_ui(world: &mut World, root: impl Fn(Ctx) -> Ctx) {
    root(Ctx {
        current_entity: world.spawn().id(),
        world,
    });
}
