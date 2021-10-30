use bevy::{
    core::FloatOrd,
    ecs::prelude::*,
    input::Input,
    prelude::{GlobalTransform, MouseButton, Touches},
    window::Windows,
};
use smallvec::SmallVec;

use crate::dom::{ClippedNode, FocusPolicy, Focusable, Focused, Interaction, Node};

pub(crate) fn focus_system(
    mut commands: Commands,
    input: Res<Input<MouseButton>>,
    q: Query<(Entity, &Interaction, Option<&Focused>), With<Focusable>>,
) {
    if input.just_pressed(MouseButton::Left) {
        for (entity, interaction, has_focused) in q.iter() {
            match (interaction, has_focused.is_some()) {
                (Interaction::Clicked, false) => {
                    commands.entity(entity).insert(Focused(()));
                }
                (Interaction::None, true) => {
                    commands.entity(entity).remove::<Focused>();
                }
                _ => {}
            }
        }
    }
}

#[derive(Default)]
pub struct State {
    entities_to_reset: SmallVec<[Entity; 1]>,
}

pub(crate) fn interaction_system(
    mut state: Local<State>,
    windows: Res<Windows>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    mut node_query: Query<(
        Entity,
        &ClippedNode,
        Option<&mut Interaction>,
        Option<&FocusPolicy>,
    )>,
) {
    let cursor_position = if let Some(cursor_position) = windows
        .get_primary()
        .and_then(|window| window.cursor_position())
    {
        cursor_position
    } else {
        return;
    };

    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(mut interaction) = node_query.get_component_mut::<Interaction>(entity) {
            *interaction = Interaction::None;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.just_released(0);
    if mouse_released {
        for (_entity, _node, interaction, _focus_policy) in node_query.iter_mut() {
            if let Some(mut interaction) = interaction {
                if *interaction == Interaction::Clicked {
                    *interaction = Interaction::None;
                }
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.just_released(0);

    let mut moused_over_z_sorted_nodes = node_query
        .iter_mut()
        .filter_map(|(entity, node, interaction, focus_policy)| {
            // if the current cursor position is within the bounds of the node, consider it for
            // clicking
            if (node.min.x..node.max.x).contains(&cursor_position.x)
                && (node.min.y..node.max.y).contains(&cursor_position.y)
            {
                Some((entity, focus_policy, interaction, node.z_layer))
            } else {
                if let Some(mut interaction) = interaction {
                    if *interaction == Interaction::Hovered {
                        *interaction = Interaction::None;
                    }
                }
                None
            }
        })
        .collect::<Vec<_>>();

    moused_over_z_sorted_nodes.sort_by_key(|(_, _, _, z)| -(*z as i32));

    let mut moused_over_z_sorted_nodes = moused_over_z_sorted_nodes.into_iter();
    // set Clicked or Hovered on top nodes
    for (entity, focus_policy, interaction, _) in moused_over_z_sorted_nodes.by_ref() {
        if let Some(mut interaction) = interaction {
            if mouse_clicked {
                // only consider nodes with Interaction "clickable"
                if *interaction != Interaction::Clicked {
                    *interaction = Interaction::Clicked;
                    // if the mouse was simultaneously released, reset this Interaction in the next
                    // frame
                    if mouse_released {
                        state.entities_to_reset.push(entity);
                    }
                }
            } else if *interaction == Interaction::None {
                *interaction = Interaction::Hovered;
            }
        }

        match focus_policy.cloned().unwrap_or(FocusPolicy::Block) {
            FocusPolicy::Block => {
                break;
            }
            FocusPolicy::Pass => { /* allow the next node to be hovered/clicked */ }
        }
    }
    // reset lower nodes to None
    for (_entity, _focus_policy, interaction, _) in moused_over_z_sorted_nodes {
        if let Some(mut interaction) = interaction {
            if *interaction != Interaction::None {
                *interaction = Interaction::None;
            }
        }
    }
}
