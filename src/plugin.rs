use bevy::app::Plugin;
use bevy::ecs::prelude::*;
use bevy::prelude::CoreStage;

use crate::animation::RunningTweens;
use crate::button::ButtonSystemState;
use crate::ctx::Ctx;
use crate::dom::layout::LayoutScratchpad;
use crate::dom::NodeBundle;
use crate::runtime::{primary_ui_system, UiManagedSystems, UiScratchSpace};
use crate::textbox::TextBoxSystemState;

#[derive(SystemLabel, Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct LayoutSystemLabel;

pub struct Ui4Plugin;
impl Plugin for Ui4Plugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<UiScratchSpace>()
            .init_resource::<ButtonSystemState>()
            .init_resource::<TextBoxSystemState>()
            .init_resource::<RunningTweens>()
            .init_resource::<LayoutScratchpad>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(primary_ui_system.exclusive_system().at_end())
            .add_system(crate::textbox::focus_system)
            .add_system(crate::animation::tween_system)
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::animation::transition_system.before(LayoutSystemLabel),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::dom::layout::root_node_system.before(LayoutSystemLabel),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::dom::layout::layout_node_system.label(LayoutSystemLabel),
            );
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
        current_entity: world
            .spawn()
            .insert_bundle(NodeBundle::default())
            .insert(crate::prelude::Width(crate::prelude::Units::Auto))
            .insert(crate::prelude::Height(crate::prelude::Units::Auto))
            .id(),
        world,
    });
}
