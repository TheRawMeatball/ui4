use bevy::app::Plugin;
use bevy::ecs::prelude::*;
use bevy::prelude::CoreStage;
use bevy_inspector_egui::RegisterInspectable;

use crate::animation::RunningTweens;
use crate::ctx::Ctx;
use crate::dom::NodeBundle;
use crate::runtime::{primary_ui_system, UiManagedSystems, UiScratchSpace};
use crate::widgets::{
    button::ButtonSystemState, slider::SliderSystemState, textbox::TextBoxSystemState,
};

#[derive(SystemLabel, Clone, Copy, Hash, PartialEq, Eq, Debug)]
enum Ui4SystemLabels {
    Layout,
    Shaping,
    Interaction,
}

pub struct Ui4Plugin;
impl Plugin for Ui4Plugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<UiScratchSpace>()
            .init_resource::<ButtonSystemState>()
            .init_resource::<TextBoxSystemState>()
            .init_resource::<RunningTweens>()
            .init_resource::<SliderSystemState>()
            .register_inspectable::<crate::dom::Node>()
            .register_inspectable::<crate::dom::ClippedNode>()
            .register_inspectable::<crate::dom::Text>()
            .register_inspectable::<crate::dom::TextSize>()
            .register_inspectable::<crate::dom::Color>()
            .register_inspectable::<crate::dom::Interaction>()
            .register_inspectable::<crate::dom::layout::layout_components::PositionType>()
            .register_inspectable::<crate::dom::layout::layout_components::LayoutType>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(crate::input::interaction_system.label(Ui4SystemLabels::Interaction))
            .add_system(crate::input::focus_system.after(Ui4SystemLabels::Interaction))
            .add_system(crate::animation::tween_system)
            .add_system(SliderSystemState::system.exclusive_system().at_end())
            .add_system(primary_ui_system.exclusive_system().at_end())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::animation::transition_system.before(Ui4SystemLabels::Layout),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::dom::layout::root_node_system.before(Ui4SystemLabels::Layout),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::dom::layout::layout_node_system.label(Ui4SystemLabels::Layout),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::dom::render::create_shapes_system
                    .after(Ui4SystemLabels::Layout)
                    .after(bevy_egui::EguiSystem::ProcessOutput)
                    .label(Ui4SystemLabels::Shaping),
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
