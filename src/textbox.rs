use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Component)]
pub struct TextBox;
#[derive(Component, Clone)]
pub struct TextBoxFunc(Arc<dyn Fn(String, &mut World) + Send + Sync>);

impl TextBoxFunc {
    pub fn new(f: impl Fn(String, &mut World) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    fn run(&self, string: String, world: &mut World) {
        (self.0)(string, world);
    }
}

#[derive(Component)]
pub struct Focused;

pub(crate) struct TextBoxSystemState {
    state: SystemState<(
        EventReader<'static, 'static, ReceivedCharacter>,
        Query<'static, 'static, &'static TextBoxFunc, With<Focused>>,
    )>,
}

impl FromWorld for TextBoxSystemState {
    fn from_world(world: &mut World) -> Self {
        TextBoxSystemState {
            state: SystemState::new(world),
        }
    }
}

impl TextBoxSystemState {
    pub(crate) fn run(&mut self, world: &mut World) {
        let (mut reader, q) = self.state.get_mut(world);
        let string = reader.iter().map(|rc| rc.char).collect::<String>();
        if let Some(tbf) = q.get_single().ok() {
            if !string.is_empty() {
                tbf.clone().run(string, world);
            }
        }
    }
}

pub(crate) fn focus_textbox_system(
    mut commands: Commands,
    input: Res<Input<MouseButton>>,
    q: Query<(Entity, &Interaction, Option<&Focused>), With<TextBox>>,
) {
    if input.just_pressed(MouseButton::Left) {
        for (entity, interaction, has_focused) in q.iter() {
            match (interaction, has_focused.is_some()) {
                (Interaction::Clicked, false) => {
                    commands.entity(entity).insert(Focused);
                }
                (Interaction::None, true) => {
                    commands.entity(entity).remove::<Focused>();
                }
                _ => {}
            }
        }
    }
}
