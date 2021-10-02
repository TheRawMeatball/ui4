use bevy::app::Plugin;
use bevy::ecs::prelude::*;

use crate::button::ButtonSystemState;
use crate::ctx::Ctx;
use crate::runtime::{primary_ui_system, UiManagedSystems, UiScratchSpace};
use crate::textbox::TextBoxSystemState;

pub struct Ui4Plugin<F>(pub F);
impl<F: Fn(Ctx) -> Ctx + Clone + Send + Sync + 'static> Plugin for Ui4Plugin<F> {
    fn build(&self, app: &mut bevy::prelude::App) {
        let root = self.0.clone();
        app.init_resource::<UiScratchSpace>()
            .init_resource::<ButtonSystemState>()
            .init_resource::<TextBoxSystemState>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(primary_ui_system.exclusive_system().at_end())
            .add_system(crate::textbox::focus_textbox_system)
            .add_startup_system(
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
