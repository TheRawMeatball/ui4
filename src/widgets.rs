pub(crate) mod button;
pub(crate) mod slider;
pub(crate) mod textbox;

use std::hash::Hash;
use std::sync::Arc;

use bevy::render2::color::Color;
use bevy::utils::HashMap;
use bevy::window::Windows;

use crate::dom::{FocusPolicy, Focusable, Node, TextBoxCursor};
use crate::{dom::Interaction, prelude::*};

use self::button::FuncScratch;
use self::slider::EngagedSlider;
use self::textbox::{TextBox, TextBoxFunc};

pub fn text<O: IntoObserver<String, M>, M>(text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with(FocusPolicy::Pass).with(
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

pub fn textbox<L: WorldLens<Out = String>>(text: L) -> impl FnOnce(Ctx) -> Ctx where {
    move |ctx: Ctx| {
        let cursor = ctx.component::<TextBox>();
        let focused = ctx.has_component::<Focused>();

        ctx.with(Width(Units::Pixels(250.)))
            .with(Height(Units::Pixels(30.)))
            .with(TextBox(0))
            .with(Focusable)
            .with(Interaction::None)
            .with(TextBoxFunc::new(move |w| text.get_mut(w)))
            .with(UiColor(Color::DARK_GRAY))
            .child(|ctx: Ctx| {
                ctx.with(FocusPolicy::Pass)
                    .with_modified::<_, L, _>(Text("".to_string()), text, |text, Text(mut old)| {
                        old.clear();
                        old.push_str(text);
                        Text(old)
                    })
                    .with(
                        cursor
                            .and(focused)
                            .map(|(c, f): (&TextBox, bool)| TextBoxCursor(f.then(|| c.0))),
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
        .with(OnClick::new(move |w| {
            let val = checked.get_mut(w);
            *val = !*val;
        }))
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
        .with(OnClick::new(move |w| {
            let val = item.get_mut(w);
            *val = this.clone();
        }))
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
            .with(Focusable)
            .children(is_open.map_child(move |b: bool| {
                let options = Arc::clone(&options);
                move |ctx: &mut McCtx| {
                    if b {
                        ctx.c(move |ctx| {
                            ctx.with(PositionType::SelfDirected)
                                .with(Height(Units::Auto))
                                .with(Bottom(Units::Percentage(100.)))
                                .with(Top(Units::Auto))
                                .children(move |ctx: &mut McCtx| {
                                    let wl = item;
                                    for (item, display) in &*options {
                                        let display: &'static str = display;
                                        let item = item.clone();
                                        ctx.c(|ctx| {
                                            button(display)(ctx).with(OnClick::new(move |w| {
                                                let m_item = wl.get_mut(w);
                                                *m_item = item.clone();
                                            }))
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
                ctx.with(Height(Units::Percentage(100.)))
                    .with(UiColor(Color::WHITE))
                    .with(
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
            // bar
            .child(|ctx: Ctx| {
                ctx.with(Height(Units::Percentage(100.)))
                    .with(
                        percent
                            .copied()
                            .map(|f: f32| Width(Units::Percentage(f * 100.))),
                    )
                    .with(MinWidth(Units::Pixels(0.)))
                    .with(UiColor(Color::WHITE))
                    .with(ChildLeft(Units::Stretch(1.)))
                    // handle
                    .child(|ctx: Ctx| {
                        let interaction = ctx.component();
                        let cursor_entity = ctx.current_entity();
                        ctx.with(Interaction::None)
                            .with(Width(Units::Pixels(15.)))
                            .with(Height(Units::Percentage(100.)))
                            .with(Right(Units::Pixels(-7.5)))
                            .with(
                                interaction.map(|interaction: &Interaction| match interaction {
                                    Interaction::Clicked => UiColor(Color::WHITE),
                                    Interaction::Hovered => UiColor(Color::GRAY),
                                    Interaction::None => UiColor(Color::GRAY),
                                }),
                            )
                            .with(FuncScratch::default())
                            .with(OnClick::new(move |w| {
                                if let Some(cursor_pos) = (|| {
                                    w.get_resource::<Windows>()?
                                        .get_primary()?
                                        .cursor_position()
                                })() {
                                    let mut cursor = w.entity_mut(cursor_entity);
                                    let cursor_node = cursor.get::<Node>().unwrap();
                                    let pos = cursor_node.pos + cursor_node.size / 2.;
                                    cursor.insert(EngagedSlider {
                                        initial_offset: cursor_pos - pos,
                                        slider_entity,
                                        get_percent: Arc::new(move |w| percent.get_mut(w)),
                                    });
                                }
                            }))
                            .with(OnRelease::new(move |w| {
                                w.entity_mut(cursor_entity).remove::<EngagedSlider>();
                            }))
                    })
            })
    }
}
