pub mod slider;
pub mod textbox;

use std::hash::Hash;
use std::sync::Arc;

use bevy::prelude::{Color, GlobalTransform, MouseButton};
use bevy::utils::HashMap;
use bevy::window::Windows;
use bevy::{ecs::prelude::*, input::Input};

use crate::{dom::Interaction, prelude::*};

use self::slider::EngagedSlider;
use self::textbox::{TextBox, TextBoxFunc};

#[derive(Component)]
pub struct Focused;
#[derive(Component)]
pub struct Focusable;

pub(crate) fn focus_system(
    mut commands: Commands,
    input: Res<Input<MouseButton>>,
    q: Query<(Entity, &Interaction, Option<&Focused>), With<Focusable>>,
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

pub fn text<O: IntoObserver<String, M>, M>(text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with(
            text.into_observer()
                .map(|text: ObsReturn<'_, _, _, O>| Text(text.borrow().clone())),
        )
    }
}

pub fn text_fade<O: IntoObserver<String, M>, M>(_text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        let transition = ctx.component().map(TransitionProgress::progress);
        text(_text)(ctx)
            .with_bundle(TransitionBundle::bidirectional(10.))
            .with(transition.map(|opacity| UiColor(Color::rgba(1., 1., 1., opacity))))
    }
}

pub fn button<O: IntoObserver<String, M>, M>(t: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        let component = ctx.component();
        ctx.with(Interaction::None)
            .with(Height(Units::Pixels(30.)))
            .with(
                component.map(|interaction: &Interaction| match interaction {
                    Interaction::Clicked => UiColor(Color::SILVER),
                    Interaction::Hovered => UiColor(Color::GRAY),
                    Interaction::None => UiColor(Color::DARK_GRAY),
                }),
            )
            .with(FuncScratch::default())
            .child(text(t))
    }
}

pub fn textbox(text: impl WorldLens<Out = String>) -> impl FnOnce(Ctx) -> Ctx where {
    move |ctx: Ctx| {
        let has_focused = ctx.has_component::<Focused>();
        let cursor = ctx.component();

        ctx.with(Width(Units::Pixels(250.)))
            .with(Height(Units::Pixels(30.)))
            .with(TextBox(0))
            .with(Focusable)
            .with(TextBoxFunc::new(move |w| text.get_mut(w)))
            .with(UiColor(Color::DARK_GRAY))
            .child(|ctx: Ctx| {
                ctx.with(
                    text.and(
                        has_focused
                            .and(cursor)
                            .map(|(focused, cursor): (bool, &TextBox)| focused.then(|| cursor.0)),
                    )
                    .map(move |(text, _cursor): (&String, Option<usize>)| {
                        let text: &str = &text.borrow();
                        // if let Some(cursor) = cursor {
                        //     Text {
                        //         sections: vec![
                        //             TextSection {
                        //                 value: text.get(..cursor).unwrap_or("").to_owned(),
                        //                 style: assets.text_style.clone(),
                        //             },
                        //             TextSection {
                        //                 value: text
                        //                     .get(cursor..cursor + 1)
                        //                     .map(|c| if c == " " { "_" } else { c })
                        //                     .unwrap_or("_")
                        //                     .to_string(),
                        //                 style: TextStyle {
                        //                     color: Color::BLACK,
                        //                     ..assets.text_style.clone()
                        //                 },
                        //             },
                        //             TextSection {
                        //                 value: text
                        //                     .get(cursor + 1..)
                        //                     .unwrap_or("")
                        //                     .to_owned(),
                        //                 style: assets.text_style.clone(),
                        //             },
                        //         ],
                        //         alignment: Default::default(),
                        //     }
                        // } else {
                        //     Text::with_section(
                        //         text.borrow(),
                        //         assets.text_style.clone(),
                        //         Default::default(),
                        //     )
                        // }
                        Text(text.to_owned())
                    }),
                )
            })
    }
}

pub fn checkbox(checked: impl WorldLens<Out = bool>) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx| {
        button(
            checked
                .copied()
                .dedup()
                .map(|b: &bool| if *b { "x" } else { " " })
                .map(|s: &'static str| s.to_string()),
        )(ctx)
        .with(ClickFunc(ButtonFunc::new(move |w| {
            let val = checked.get_mut(w);
            *val = !*val;
        })))
    }
}

pub fn radio_button<T>(this: T, item: impl WorldLens<Out = T>) -> impl FnOnce(Ctx) -> Ctx
where
    T: PartialEq + Clone + Send + Sync + 'static,
{
    let this1 = this.clone();
    move |ctx| {
        button(
            item.cloned()
                .dedup()
                .map(move |t: &T| if t == &this1 { "x" } else { " " })
                .map(|s: &'static str| s.to_string()),
        )(ctx)
        .with(ClickFunc(ButtonFunc::new(move |w| {
            let val = item.get_mut(w);
            *val = this.clone();
        })))
    }
}

pub fn dropdown<T, const N: usize>(
    options: [(T, &'static str); N],
    item: impl WorldLens<Out = T>,
) -> impl FnOnce(Ctx) -> Ctx
where
    T: Eq + Hash + Clone + Send + Sync + 'static,
{
    let options_map: HashMap<_, _> = options.iter().cloned().collect();
    let options = Arc::new(options);

    move |ctx| {
        let is_open = ctx.has_component::<Focused>();

        button(item.map(move |s: &T| options_map[s].to_string()))(ctx)
            .with(Height(Units::Pixels(30.)))
            .with(Focusable)
            .children(is_open.map_child(move |b: bool| {
                let options = Arc::clone(&options);
                move |ctx: &mut McCtx| {
                    if b {
                        ctx.c(move |ctx| {
                            ctx.with(PositionType::SelfDirected)
                                .with(Top(Units::Percentage(100.)))
                                .children(move |ctx: &mut McCtx| {
                                    let wl = item;
                                    for (item, display) in &*options {
                                        let display: &'static str = display;
                                        let item = item.clone();
                                        ctx.c(|ctx| {
                                            button(display)(ctx).with(ClickFunc(ButtonFunc::new(
                                                move |w| {
                                                    let m_item = wl.get_mut(w);
                                                    *m_item = item.clone();
                                                },
                                            )))
                                        });
                                    }
                                })
                        });
                    }
                }
            }))
    }
}

pub fn progressbar<O: IntoObserver<f32, M>, M>(percent: O) -> impl FnOnce(Ctx) -> Ctx {
    |ctx| {
        ctx.with(Width(Units::Pixels(250.)))
            .with(Height(Units::Pixels(30.)))
            .with(UiColor(Color::DARK_GRAY))
            .child(|ctx: Ctx| {
                ctx.with(Height(Units::Auto)).with(
                    percent
                        .into_observer()
                        .map(|f: ObsReturn<'_, _, _, O>| *f.borrow())
                        .map(|f: f32| Width(Units::Percentage(f * 100.))),
                )
            })
    }
}

pub fn slider(percent: impl WorldLens<Out = f32>) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx| {
        let slider_entity = ctx.current_entity();
        ctx.with(Width(Units::Pixels(250.)))
            .with(Height(Units::Pixels(30.)))
            .with(UiColor(Color::DARK_GRAY))
            .child(|ctx: Ctx| {
                ctx.with(Height(Units::Auto))
                    .with(
                        percent
                            .copied()
                            .map(|f: f32| Width(Units::Percentage(f * 100.))),
                    )
                    .with(UiColor(Color::GRAY))
                    .child(|ctx: Ctx| {
                        let interaction = ctx.component();
                        let cursor_entity = ctx.current_entity();
                        ctx.with(Interaction::None)
                            .with(Width(Units::Pixels(20.)))
                            .with(Height(Units::Auto))
                            .with(Right(Units::Pixels(-10.)))
                            .with(
                                interaction.map(|interaction: &Interaction| match interaction {
                                    Interaction::Clicked => UiColor(Color::WHITE),
                                    Interaction::Hovered => UiColor(Color::GRAY),
                                    Interaction::None => UiColor(Color::GRAY),
                                }),
                            )
                            .with(FuncScratch::default())
                            .with(ClickFunc(ButtonFunc::new(move |w| {
                                if let Some(cursor_pos) = (|| {
                                    w.get_resource::<Windows>()?
                                        .get_primary()?
                                        .cursor_position()
                                })() {
                                    let mut cursor = w.entity_mut(cursor_entity);
                                    let pos = cursor
                                        .get::<GlobalTransform>()
                                        .unwrap()
                                        .translation
                                        .truncate();
                                    cursor.insert(EngagedSlider {
                                        initial_offset: cursor_pos - pos,
                                        slider_entity,
                                        get_percent: Arc::new(move |w| percent.get_mut(w)),
                                    });
                                }
                            })))
                            .with(ReleaseFunc(ButtonFunc::new(move |w| {
                                w.entity_mut(cursor_entity).remove::<EngagedSlider>();
                            })))
                    })
            })
    }
}
