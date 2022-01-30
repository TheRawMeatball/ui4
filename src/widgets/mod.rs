//! The ui4 built in widget library

pub mod button;
pub(crate) mod draggable;
pub(crate) mod textbox;

use std::hash::Hash;
use std::sync::Arc;

use bevy::math::Vec2;
use bevy::render::color::Color;
use bevy::text::TextAlignment;
use bevy::ui::UiColor;
use bevy::utils::HashMap;
use bevy::window::Windows;

use crate::dom::{FocusPolicy, Focusable, Node, TextBoxCursor, UiText};
use crate::{dom::Interaction, prelude::*};

use self::button::{ClickColor, FuncScratch, HoverColor, NormalColor};
use self::draggable::EngagedDraggable;
use self::textbox::{TextBox, TextBoxFunc};

pub fn text<O: IntoObserver<String, M>, M>(text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with(FocusPolicy::Pass).with(
            text.into_observer()
                .map(|text: ObsReturn<'_, _, _, O>| UiText(text.borrow().clone())),
        )
    }
}

pub fn text_fade<O: IntoObserver<String, M>, M>(_text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        let transition = ctx.component().map(TransitionProgress::progress);
        ctx.inherit(text(_text))
            .with_bundle(TransitionBundle::bidirectional(10.))
            .with(transition.map(|opacity| UiColor(Color::rgba(1., 1., 1., opacity))))
    }
}

pub fn button<O: IntoObserver<String, M>, M: 'static>(t: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        let component = ctx.component();
        let hover = ctx
            .opt_component()
            .map(|x: Option<&HoverColor>| x.map_or(Color::GRAY, |x| x.0));
        let normal = ctx
            .opt_component()
            .map(|x: Option<&NormalColor>| x.map_or(Color::DARK_GRAY, |x| x.0));
        let click = ctx
            .opt_component()
            .map(|x: Option<&ClickColor>| x.map_or(Color::SILVER, |x| x.0));

        let text_size = ctx.opt_component().map(|x: Option<&TextSize>| x.copied());

        let color = hover.and(normal).and(click).map(|((h, n), c)| (h, n, c));

        ctx.with(Interaction::None)
            .with(Height(Units::Pixels(30.)))
            .with(
                component
                    .and(color)
                    .map(
                        |(interaction, (h, n, c)): (&Interaction, _)| match interaction {
                            Interaction::Clicked => UiColor(c),
                            Interaction::Hovered => UiColor(h),
                            Interaction::None => UiColor(n),
                        },
                    ),
            )
            .with(FuncScratch::default())
            .child(
                text(t)
                    .with(TextAlign(TextAlignment {
                        vertical: bevy::text::VerticalAlign::Center,
                        horizontal: bevy::text::HorizontalAlign::Center,
                    }))
                    .with(text_size),
            )
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
                    .with_modified::<_, L, _>(
                        UiText("".to_string()),
                        text,
                        |text, UiText(mut old)| {
                            old.clear();
                            old.push_str(text);
                            UiText(old)
                        },
                    )
                    .with(
                        cursor
                            .and(focused)
                            .map(|(c, f): (&TextBox, bool)| TextBoxCursor(f.then(|| c.0))),
                    )
            })
    }
}

pub fn checkbox(checked: impl WorldLens<Out = bool>) -> impl FnOnce(Ctx) -> Ctx {
    button(
        checked
            .copied()
            .dedup()
            .map(|b: &bool| if *b { "X" } else { " " })
            .map(|s: &'static str| s.to_string()),
    )
    .with(OnClick::new(move |w| {
        let val = checked.get_mut(w);
        *val = !*val;
    }))
}

pub fn radio_button<T>(this: T, item: impl WorldLens<Out = T>) -> impl FnOnce(Ctx) -> Ctx
where
    T: PartialEq + Clone + Send + Sync + 'static,
{
    let this1 = this.clone();
    button(
        item.cloned()
            .dedup()
            .map(move |t: &T| if t == &this1 { "x" } else { " " })
            .map(|s: &'static str| s.to_string()),
    )
    .with(OnClick::new(move |w| {
        let val = item.get_mut(w);
        *val = this.clone();
    }))
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

    move |ctx: Ctx| {
        let is_open = ctx.has_component::<Focused>();

        ctx.inherit(button(item.map(move |s: &T| options_map[s].to_string())))
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
                                        ctx.c(button(display).with(OnClick::new(move |w| {
                                            let m_item = wl.get_mut(w);
                                            *m_item = item.clone();
                                        })));
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
                                    let initial_offset = cursor_pos - pos;
                                    cursor.insert(EngagedDraggable {
                                        process: Arc::new(move |w, cursor_pos| {
                                            let node = w.get::<Node>(slider_entity).unwrap();
                                            let len = node.size.x;
                                            let start = node.pos.x;
                                            let current = cursor_pos.x - initial_offset.x;
                                            let p = ((current - start) / len).clamp(0., 1.);
                                            *percent.get_mut(w) = p;
                                        }),
                                    });
                                }
                            }))
                            .with(OnRelease::new(move |w| {
                                w.entity_mut(cursor_entity).remove::<EngagedDraggable>();
                            }))
                    })
            })
    }
}

pub fn vscroll_view<M>(inner: impl Childable<M>) -> impl FnOnce(Ctx) -> Ctx {
    |ctx: Ctx| {
        let avail_height = ctx.component().map(|node: &Node| node.size.y);
        let mut content_height = None;
        let mut container_entity = None;
        let ctx = ctx.with(LayoutType::Row).child(|ctx| {
            ctx.with(HideOverflow)
                // .with(UiColor(Color::RED))
                .with(MinHeight(Units::Pixels(0.)))
                .with(Height(Units::Percentage(100.)))
                .child(|ctx| {
                    let ch = ctx.component().map(|node: &Node| node.size.y);
                    content_height = Some(ch);
                    let heights_obs = ch.and(avail_height);
                    container_entity = Some(ctx.current_entity());
                    ctx.with_modified(
                        Top(Units::Pixels(0.)),
                        heights_obs,
                        |(content, available), Top(pre)| match pre {
                            Units::Pixels(pre) => {
                                Top(Units::Pixels(pre.max((available - content).min(0.))))
                            }
                            _ => unreachable!(),
                        },
                    )
                    // .with(UiColor(Color::GREEN))
                    .with(Height(Units::Pixels(0.)))
                    .children(inner)
                })
        });
        let container_entity = container_entity.unwrap();
        let content_height = content_height.unwrap();

        let heights_obs = content_height.and(avail_height);
        let need_scroll_obs = heights_obs.map(|(c, a)| c > a);
        ctx.children(need_scroll_obs.map_child(move |ratio_over_one: bool| {
            move |ctx: &mut McCtx| {
                if ratio_over_one {
                    ctx.c(|ctx| {
                        let scroll_entity = ctx.current_entity();
                        ctx.with(UiColor(Color::DARK_GRAY))
                            .with(Width(Units::Pixels(12.)))
                            .child(|ctx| {
                                let cursor_entity = ctx.current_entity();
                                ctx.with(UiColor(Color::GRAY))
                                    .with_modified(
                                        Top(Units::Pixels(0.)),
                                        heights_obs,
                                        |(content, available), Top(pre)| match pre {
                                            Units::Pixels(pre) => Top(Units::Pixels(
                                                pre.min(
                                                    (available - (available * available / content))
                                                        .max(0.),
                                                ),
                                            )),
                                            _ => unreachable!(),
                                        },
                                    )
                                    .with(
                                        heights_obs
                                            .map(|(c, a)| 100. * a / c)
                                            .map(Units::Percentage)
                                            .map(Height),
                                    )
                                    .with(OnClick::new(move |w| {
                                        if let Some((cursor_pos, height)) = (|| {
                                            let window =
                                                w.get_resource::<Windows>()?.get_primary()?;
                                            Some((window.cursor_position()?, window.height()))
                                        })(
                                        ) {
                                            let mut cursor = w.entity_mut(cursor_entity);
                                            let cursor_node = cursor.get::<Node>().unwrap();
                                            let pos = cursor_node.pos.y;
                                            let initial_offset = height - cursor_pos.y - pos;
                                            cursor.insert(EngagedDraggable {
                                                process: Arc::new(move |w, cursor_pos| {
                                                    let scroll_node =
                                                        *w.get::<Node>(scroll_entity).unwrap();
                                                    let cursor_node =
                                                        *w.get::<Node>(cursor_entity).unwrap();
                                                    let len =
                                                        scroll_node.size.y - cursor_node.size.y;
                                                    let start = scroll_node.pos.y;
                                                    let current =
                                                        height - cursor_pos.y - initial_offset;
                                                    let p = ((current - start) / len).clamp(0., 1.);
                                                    w.get_mut::<Top>(cursor_entity).unwrap().0 =
                                                        Units::Pixels(p * len);
                                                    let container_node =
                                                        *w.get::<Node>(container_entity).unwrap();
                                                    w.get_mut::<Top>(container_entity).unwrap().0 =
                                                        Units::Pixels(
                                                            p * (scroll_node.size.y
                                                                - container_node.size.y),
                                                        );
                                                }),
                                            });
                                        }
                                    }))
                                    .with(OnRelease::new(move |w| {
                                        w.entity_mut(cursor_entity).remove::<EngagedDraggable>();
                                    }))
                                    .with(Interaction::None)
                                    .with(FuncScratch::default())
                            })
                    });
                }
            }
        }))
    }
}

/// A window you can drag around in it's parent
///
/// Use in combination with [ManualRoot](crate::dom::ManualRoot) if you want to make a popup.
pub fn draggable_window<M>(inner: impl Childable<M>) -> impl FnOnce(Ctx) -> Ctx {
    |ctx: Ctx| {
        let main_box = ctx.current_entity();
        ctx.with(PositionType::SelfDirected)
            .with(Top(Units::Pixels(0.)))
            .with(Left(Units::Pixels(0.)))
            .child(|ctx| {
                let drag_box_entity = ctx.current_entity();
                ctx.with(UiColor(Color::BLACK))
                    .with(MinWidth(Units::Pixels(200.)))
                    .with(Width(Units::Auto))
                    .with(Height(Units::Pixels(20.)))
                    .with(OnClick::new(move |w| {
                        if let Some((cursor_pos, height)) = (|| {
                            let window = w.get_resource::<Windows>()?.get_primary()?;
                            Some((window.cursor_position()?, window.height()))
                        })() {
                            // preprocessing
                            let drag_box = w.entity(main_box);
                            let initial_pos = Vec2::Y * height + cursor_pos * Vec2::new(1., -1.);
                            let t = match drag_box.get::<Top>() {
                                Some(Top(Units::Pixels(p))) => *p,
                                _ => unreachable!(),
                            };
                            let l = match drag_box.get::<Left>() {
                                Some(Left(Units::Pixels(p))) => *p,
                                _ => unreachable!(),
                            };
                            let initial_lt = Vec2::new(l, t);
                            w.entity_mut(drag_box_entity).insert(EngagedDraggable {
                                process: Arc::new(move |w, cursor_pos| {
                                    // processing
                                    let cursor_pos =
                                        Vec2::Y * height + cursor_pos * Vec2::new(1., -1.);
                                    let delta = cursor_pos - initial_pos;
                                    let new_lt = initial_lt + delta;
                                    let mut drag_box = w.entity_mut(main_box);
                                    drag_box.insert(Left(Units::Pixels(new_lt.x)));
                                    drag_box.insert(Top(Units::Pixels(new_lt.y)));
                                }),
                            });
                        }
                    }))
                    .with(OnRelease::new(move |w| {
                        w.entity_mut(drag_box_entity).remove::<EngagedDraggable>();
                    }))
                    .with(Interaction::None)
                    .with(FuncScratch::default())
            })
            .child(|ctx| {
                ctx.with(MinWidth(Units::Pixels(200.)))
                    .with(Width(Units::Auto))
                    .with(MinHeight(Units::Pixels(180.)))
                    .with(Height(Units::Auto))
                    .with(UiColor(Color::DARK_GRAY))
                    .children(inner)
            })
    }
}
