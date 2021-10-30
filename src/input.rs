use bevy::{ecs::prelude::*, input::Input, prelude::MouseButton};

use crate::dom::{Focusable, Focused, Interaction};

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

pub(crate) fn interaction_system() {}
