use bevy::app::Plugin;
use bevy::ecs::prelude::*;

use crate::button::ButtonSystemState;
use crate::ctx::Ctx;
use crate::runtime::{primary_ui_system, UiManagedSystems, UiScratchSpace};

pub struct Ui4Plugin<F>(pub F);
impl<F: Fn(&mut Ctx) + Clone + Send + Sync + 'static> Plugin for Ui4Plugin<F> {
    fn build(&self, app: &mut bevy::prelude::App) {
        let root = self.0.clone();
        app.init_resource::<UiScratchSpace>()
            .init_resource::<ButtonSystemState>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(primary_ui_system.exclusive_system().at_end())
            .add_startup_system(
                (move |world: &mut World| init_ui(world, &root))
                    .exclusive_system()
                    .at_end(),
            );
    }
}

fn init_ui(world: &mut World, root: impl Fn(&mut Ctx)) {
    root(&mut Ctx {
        current_entity: world.spawn().id(),
        world,
    })
}
