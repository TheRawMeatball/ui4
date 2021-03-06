use bevy::app::Plugin;
use bevy::ecs::prelude::*;
use bevy::prelude::{Assets, CoreStage};
use bevy::text::Font;
use bevy_inspector_egui::RegisterInspectable;

use crate::ctx::Ctx;
use crate::dom::render::PreExtractedUiNodes;
use crate::dom::NodeBundle;
use crate::runtime::{primary_ui_system, UiManagedSystems, UiScratchSpace};
use crate::widgets::{
    button::ButtonSystemState, draggable::DraggableSystemState, textbox::TextBoxSystemState,
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
        if !app
            .world
            .contains_resource::<bevy_inspector_egui::InspectableRegistry>()
        {
            app.init_resource::<bevy_inspector_egui::InspectableRegistry>();
        }

        app.init_resource::<UiScratchSpace>()
            .init_resource::<ButtonSystemState>()
            .init_resource::<TextBoxSystemState>()
            .init_resource::<DraggableSystemState>()
            .init_resource::<PreExtractedUiNodes>()
            .register_inspectable::<crate::dom::Node>()
            .register_inspectable::<crate::dom::ClippedNode>()
            .register_inspectable::<crate::dom::UiText>()
            .register_inspectable::<crate::dom::TextSize>()
            .register_inspectable::<crate::dom::Interaction>()
            .register_inspectable::<crate::dom::TextAlign>()
            .register_inspectable::<crate::dom::TextDetails>()
            .register_inspectable::<crate::dom::layout::layout_components::PositionType>()
            .register_inspectable::<crate::dom::layout::layout_components::LayoutType>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(crate::input::interaction_system.label(Ui4SystemLabels::Interaction))
            .add_system(crate::input::focus_system.after(Ui4SystemLabels::Interaction))
            .add_system(crate::animation::tween_system)
            .add_system(DraggableSystemState::system.exclusive_system().at_end())
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
                    .label(Ui4SystemLabels::Shaping),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                crate::dom::render::process_text_system
                    .after(Ui4SystemLabels::Layout)
                    .before(Ui4SystemLabels::Shaping),
            );

        let render_app = app.sub_app_mut(bevy::render::RenderApp);
        render_app.add_system_to_stage(
            bevy::render::RenderStage::Extract,
            crate::dom::render::move_uinodes.after(bevy::ui::RenderUiSystem::ExtractNode),
        );

        crate::dom::layout::layout_components::register_all(app);

        let font =
            Font::try_from_bytes(include_bytes!("../assets/FiraMono-Medium.ttf").to_vec()).unwrap();
        app.world
            .get_resource_mut::<Assets<Font>>()
            .unwrap()
            .set_untracked(crate::dom::render::DEFAULT_FONT, font);
    }
}

pub struct Ui4Root<F>(pub F);
impl<F: FnOnce(Ctx) -> Ctx + Clone + Send + Sync + 'static> Plugin for Ui4Root<F> {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut root = Some(self.0.clone());
        app.add_startup_system(
            (move |world: &mut World| init_ui(world, root.take().unwrap()))
                .exclusive_system()
                .at_end(),
        );
    }
}

/// Initialize a new ui tree
pub fn init_ui(world: &mut World, root: impl FnOnce(Ctx) -> Ctx) {
    (Ctx {
        current_entity: world
            .spawn()
            .insert_bundle(NodeBundle::default())
            .insert(crate::prelude::Width(crate::prelude::Units::Auto))
            .insert(crate::prelude::Height(crate::prelude::Units::Auto))
            .id(),
        world,
    })
    .child(root);
}
