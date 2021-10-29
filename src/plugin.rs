use bevy::app::Plugin;
use bevy::ecs::prelude::*;
use bevy::prelude::CoreStage;
use bevy_inspector_egui::RegisterInspectable;

use crate::animation::RunningTweens;
use crate::ctx::Ctx;
use crate::dom::layout::LayoutScratchpad;
use crate::dom::NodeBundle;
use crate::runtime::{primary_ui_system, UiManagedSystems, UiScratchSpace};
use crate::widgets::{
    button::ButtonSystemState, slider::SliderSystemState, textbox::TextBoxSystemState,
};

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
            .init_resource::<SliderSystemState>()
            .register_inspectable::<crate::dom::Node>()
            .register_inspectable::<crate::dom::Text>()
            .register_inspectable::<crate::dom::TextSize>()
            .register_inspectable::<crate::dom::Color>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(SliderSystemState::system.exclusive_system())
            .add_system(primary_ui_system.exclusive_system().at_end())
            .add_system(crate::widgets::focus_system)
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
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::dom::render::create_shapes_system
                    .after(LayoutSystemLabel)
                    .after(bevy_egui::EguiSystem::ProcessOutput),
            );

        crate::dom::layout::layout_components::register_all(app);
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
