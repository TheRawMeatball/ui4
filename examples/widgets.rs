use bevy::{ecs::system::SystemState, prelude::*, utils::HashMap};
use derive_more::{Deref, DerefMut};
use std::{borrow::Borrow, hash::Hash, sync::Arc};
use ui4::prelude::*;

struct UiAssets {
    background: Handle<ColorMaterial>,
    button: Handle<ColorMaterial>,
    button_hover: Handle<ColorMaterial>,
    button_click: Handle<ColorMaterial>,
    white: Handle<ColorMaterial>,
    text_style: TextStyle,
    transparent: Handle<ColorMaterial>,
}

fn init_system(
    mut commands: Commands,
    mut assets: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(UiAssets {
        background: assets.add(Color::BLACK.into()),
        transparent: assets.add(Color::NONE.into()),
        button: assets.add(Color::DARK_GRAY.into()),
        button_hover: assets.add(Color::GRAY.into()),
        button_click: assets.add(Color::SILVER.into()),
        white: assets.add(Color::WHITE.into()),
        text_style: TextStyle {
            color: Color::WHITE,
            font: asset_server.load("FiraMono-Medium.ttf"),
            font_size: 32.0,
        },
    })
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root))
        .init_resource::<SliderSystemState>()
        .add_system(SliderSystemState::system.exclusive_system())
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_startup_system(init_system);

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component, Deref, DerefMut, Default, Lens)]
    struct TextboxText(String);

    #[derive(Component, Deref, DerefMut, Default, Lens)]
    struct CheckboxData(bool);

    #[derive(Component, Hash, Copy, Clone, PartialEq, Eq)]
    enum RadioButtonSelect {
        A,
        B,
        C,
    }

    #[derive(Component, Lens)]
    struct Slider(f32);

    let textbox_text = ctx.component();
    let checkbox_data = ctx.component();
    let radiobutton = ctx.component();

    let slider_percent = ctx.component();

    ctx.with_bundle(NodeBundle::default())
        .with(Style {
            size: Size {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
            },
            flex_direction: FlexDirection::ColumnReverse,
            ..Default::default()
        })
        .with(res().map(|assets: &UiAssets| assets.background.clone()))
        .with(TextboxText::default())
        .with(CheckboxData::default())
        .with(RadioButtonSelect::A)
        .with(Slider(0.42))
        .children(|ctx: &mut McCtx| {
            ctx.c(labelled_widget(
                "Button",
                button("Click me!", |_| println!("you clicked the button!")),
            ))
            .c(labelled_widget(
                "Textbox",
                textbox(textbox_text.lens(TextboxText::F0)),
            ))
            .c(labelled_widget(
                "Checkbox",
                checkbox(checkbox_data.lens(CheckboxData::F0)),
            ))
            .c(labelled_widget("Radio buttons", |ctx| {
                ctx.with_bundle(NodeBundle::default())
                    .with(Style {
                        size: Size {
                            width: Val::Px(250.),
                            height: Val::Px(30.),
                        },
                        justify_content: JustifyContent::SpaceBetween,
                        ..Default::default()
                    })
                    .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
                    .children(|ctx: &mut McCtx| {
                        ctx.c(radio_button(RadioButtonSelect::A, radiobutton))
                            .c(text("A  "))
                            .c(radio_button(RadioButtonSelect::B, radiobutton))
                            .c(text("B  "))
                            .c(radio_button(RadioButtonSelect::C, radiobutton))
                            .c(text("C  "));
                    })
            }))
            .c(labelled_widget(
                "Dropdown",
                dropdown(
                    [
                        (RadioButtonSelect::A, "A"),
                        (RadioButtonSelect::B, "B"),
                        (RadioButtonSelect::C, "C"),
                    ],
                    radiobutton,
                ),
            ))
            .c(labelled_widget(
                "Progress",
                progressbar(slider_percent.map(|f: &Slider| f.0)),
            ))
            .c(labelled_widget(
                "Slider",
                slider(slider_percent.lens(Slider::F0)),
            ))
            .c(labelled_widget(
                "Tweened",
                progressbar(
                    textbox_text
                        .map(|t: &TextboxText| t.0.parse::<f32>().unwrap_or(0.42).clamp(0., 1.))
                        .dedup()
                        .map(|x: &f32| *x)
                        .tween(0.2),
                ),
            ))
            .c(toggle(|| toggle(|| text_fade("Hey!"))));
        })
}

fn labelled_widget(
    label: &'static str,
    widget: impl FnOnce(Ctx) -> Ctx,
) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with_bundle(NodeBundle::default())
            .with(Style {
                size: Size {
                    width: Val::Px(400.),
                    height: Val::Px(30.),
                },
                ..Default::default()
            })
            .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
            .children(|ctx: &mut McCtx| {
                ctx.c(|ctx| {
                    text(label)(ctx).with(Style {
                        size: Size {
                            width: Val::Px(150.),
                            height: Val::Px(30.),
                        },
                        ..Default::default()
                    })
                })
                .c(widget);
            })
    }
}

fn toggle<F: FnOnce(Ctx) -> Ctx>(
    child: impl Fn() -> F + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx {
    #[derive(Component, Deref, DerefMut, Default, Lens)]
    struct Toggle(bool);
    |ctx: Ctx| {
        let checked = ctx.component::<Toggle>();
        ctx.with_bundle(NodeBundle::default())
            .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
            .with(Toggle(false))
            .child(checkbox(checked.lens(Toggle::F0)))
            .children(checked.map(|t: &Toggle| t.0).map_child(move |b| {
                let child = child();
                move |ctx: &mut McCtx| {
                    if b {
                        ctx.c(child);
                    }
                }
            }))
    }
}

fn text<O: IntoObserver<String, M>, M>(text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with_bundle(TextBundle::default())
            .with(Style {
                align_self: AlignSelf::FlexStart,
                ..Default::default()
            })
            .with(res().and(text.into_observer()).map(
                move |(assets, text): (&UiAssets, O::ObserverReturn<'_, '_>)| {
                    Text::with_section(text.borrow(), assets.text_style.clone(), Default::default())
                },
            ))
    }
}

fn text_fade<O: IntoObserver<String, M>, M>(text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        let transition = ctx.component().map(TransitionProgress::progress);
        ctx.with_bundle(TextBundle::default())
            .with_bundle(TransitionBundle::bidirectional(10.))
            .with(Style {
                align_self: AlignSelf::FlexStart,
                ..Default::default()
            })
            .with(
                res()
                    .and(transition)
                    .map(|(assets, opacity): (&UiAssets, f32)| TextStyle {
                        color: Color::rgba(1., 1., 1., opacity),
                        ..assets.text_style.clone()
                    })
                    .and(text.into_observer())
                    .map(
                        move |(style, text): (TextStyle, O::ObserverReturn<'_, '_>)| {
                            Text::with_section(text.borrow(), style, Default::default())
                        },
                    ),
            )
    }
}

fn button<O: IntoObserver<String, M>, M>(
    t: O,
    on_click: impl Fn(&mut World) + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        let component = ctx.component();
        ctx.with_bundle(ButtonBundle::default())
            .with(Style {
                size: Size::new(Val::Undefined, Val::Px(30.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,

                ..Default::default()
            })
            .with(
                res()
                    .and(component)
                    .map(
                        |(assets, interaction): (&UiAssets, &Interaction)| match interaction {
                            Interaction::Clicked => assets.button_click.clone(),
                            Interaction::Hovered => assets.button_hover.clone(),
                            Interaction::None => assets.button.clone(),
                        },
                    ),
            )
            .with(FuncScratch::default())
            .with(ClickFunc(ButtonFunc::new(on_click)))
            .child(text(t))
    }
}

fn textbox(text: impl WorldLens<Out = String>) -> impl FnOnce(Ctx) -> Ctx where {
    move |ctx: Ctx| {
        let has_focused = ctx.has_component::<Focused>();
        let cursor = ctx.component();

        ctx.with_bundle(ButtonBundle::default())
            .with(Style {
                size: Size::new(Val::Px(250.0), Val::Px(30.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..Default::default()
            })
            .with(TextBox(0))
            .with(Focusable)
            .with(TextBoxFunc::new(move |world| text.get_mut(world)))
            .with(res().map(|assets: &UiAssets| assets.button.clone()))
            .child(|ctx: Ctx| {
                ctx.with_bundle(TextBundle::default())
                    .with(Style {
                        align_self: AlignSelf::FlexStart,
                        ..Default::default()
                    })
                    .with(
                        res::<UiAssets>()
                            .and(text)
                            .and(has_focused.and(cursor).map(
                                |(focused, cursor): (bool, &TextBox)| focused.then(|| cursor.0),
                            ))
                            .map(
                                move |((assets, text), cursor): (
                                    (&UiAssets, &String),
                                    Option<usize>,
                                )| {
                                    let text: &str = &text.borrow();
                                    if let Some(cursor) = cursor {
                                        Text {
                                            sections: vec![
                                                TextSection {
                                                    value: text
                                                        .get(..cursor)
                                                        .unwrap_or("")
                                                        .to_owned(),
                                                    style: assets.text_style.clone(),
                                                },
                                                TextSection {
                                                    value: text
                                                        .get(cursor..cursor + 1)
                                                        .map(|c| if c == " " { "_" } else { c })
                                                        .unwrap_or("_")
                                                        .to_string(),
                                                    style: TextStyle {
                                                        color: Color::BLACK,
                                                        ..assets.text_style.clone()
                                                    },
                                                },
                                                TextSection {
                                                    value: text
                                                        .get(cursor + 1..)
                                                        .unwrap_or("")
                                                        .to_owned(),
                                                    style: assets.text_style.clone(),
                                                },
                                            ],
                                            alignment: Default::default(),
                                        }
                                    } else {
                                        Text::with_section(
                                            text.borrow(),
                                            assets.text_style.clone(),
                                            Default::default(),
                                        )
                                    }
                                },
                            ),
                    )
            })
    }
}

fn checkbox(checked: impl WorldLens<Out = bool>) -> impl FnOnce(Ctx) -> Ctx {
    button(
        checked
            .map(|&b: &bool| b)
            .dedup()
            .map(|b: &bool| if *b { "x" } else { " " })
            .map(|s: &'static str| s.to_string()),
        move |w| {
            let val = checked.get_mut(w);
            *val = !*val;
        },
    )
}

fn radio_button<T>(this: T, item: impl WorldLens<Out = T>) -> impl FnOnce(Ctx) -> Ctx
where
    T: PartialEq + Clone + Send + Sync + 'static,
{
    let this1 = this.clone();
    button(
        item.map(|t: &T| t.clone())
            .dedup()
            .map(move |t: &T| if t == &this1 { "x" } else { " " })
            .map(|s: &'static str| s.to_string()),
        move |w| {
            let val = item.get_mut(w);
            *val = this.clone();
        },
    )
}

fn dropdown<T, const N: usize>(
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

        button(
            item.map(move |s: &T| options_map[s.borrow()].to_string()),
            move |_| {},
        )(ctx)
        .with(Style {
            size: Size::new(Val::Undefined, Val::Px(30.0)),
            ..Default::default()
        })
        .with(Focusable)
        .children(is_open.map_child(move |b: bool| {
            let options = Arc::clone(&options);
            move |ctx: &mut McCtx| {
                if b {
                    ctx.c(move |ctx| {
                        ctx.with_bundle(NodeBundle::default())
                            .with(Style {
                                align_self: AlignSelf::FlexEnd,
                                position_type: PositionType::Absolute,
                                flex_direction: FlexDirection::ColumnReverse,
                                position: Rect {
                                    left: Val::Undefined,
                                    right: Val::Undefined,
                                    top: Val::Percent(100.),
                                    bottom: Val::Undefined,
                                },
                                ..Default::default()
                            })
                            .children(move |ctx: &mut McCtx| {
                                let wl = item;
                                for (item, display) in &*options {
                                    let display: &'static str = display;
                                    let item = item.clone();
                                    ctx.c(button(display, move |w| {
                                        let m_item = wl.get_mut(w);
                                        *m_item = item.clone();
                                    }));
                                }
                            })
                    });
                }
            }
        }))
    }
}

fn progressbar<O: IntoObserver<f32, M>, M>(percent: O) -> impl FnOnce(Ctx) -> Ctx {
    |ctx| {
        ctx.with_bundle(NodeBundle::default())
            .with(Style {
                size: Size::new(Val::Px(250.0), Val::Px(30.0)),
                justify_content: JustifyContent::FlexStart,
                ..Default::default()
            })
            .with(res().map(|assets: &UiAssets| assets.button.clone()))
            .child(|ctx: Ctx| {
                ctx.with_bundle(NodeBundle::default()).with(
                    percent
                        .into_observer()
                        .map(|f: O::ObserverReturn<'_, '_>| *f.borrow())
                        .map(|f: f32| Style {
                            size: Size::new(Val::Percent(f * 100.), Val::Auto),
                            justify_content: JustifyContent::FlexEnd,
                            ..Default::default()
                        }),
                )
            })
    }
}

#[derive(Component)]
struct EngagedSlider {
    initial_offset: Vec2,
    slider_entity: Entity,
    get_percent: Arc<dyn Fn(&mut World) -> &mut f32 + Send + Sync>,
}

fn slider(percent: impl WorldLens<Out = f32>) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx| {
        let slider_entity = ctx.current_entity();
        ctx.with_bundle(NodeBundle::default())
            .with(Style {
                size: Size::new(Val::Px(250.0), Val::Px(30.0)),
                justify_content: JustifyContent::FlexStart,
                ..Default::default()
            })
            .with(res().map(|assets: &UiAssets| assets.button.clone()))
            .child(|ctx: Ctx| {
                ctx.with_bundle(NodeBundle::default())
                    .with(percent.map(|&f: &f32| f).map(|f: f32| Style {
                        size: Size::new(Val::Percent(f * 100.), Val::Auto),
                        justify_content: JustifyContent::FlexEnd,
                        ..Default::default()
                    }))
                    .with(res().map(|assets: &UiAssets| assets.button_hover.clone()))
                    .child(|ctx: Ctx| {
                        let interaction = ctx.component();
                        let cursor_entity = ctx.current_entity();
                        ctx.with_bundle(ButtonBundle::default())
                            .with(Style {
                                position: Rect {
                                    left: Val::Undefined,
                                    right: Val::Px(-10.),
                                    top: Val::Undefined,
                                    bottom: Val::Undefined,
                                },
                                size: Size {
                                    width: Val::Px(20.),
                                    height: Val::Auto,
                                },
                                flex_shrink: 0.,
                                ..Default::default()
                            })
                            .with(res().and(interaction).map(
                                |(assets, interaction): (&UiAssets, &Interaction)| match interaction
                                {
                                    Interaction::Clicked => assets.white.clone(),
                                    Interaction::Hovered => assets.button_click.clone(),
                                    Interaction::None => assets.button_click.clone(),
                                },
                            ))
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

struct SliderSystemState {
    state: SystemState<(
        Query<'static, 'static, &'static EngagedSlider>,
        Query<'static, 'static, (&'static Node, &'static GlobalTransform)>,
        Res<'static, Windows>,
    )>,
}

impl FromWorld for SliderSystemState {
    fn from_world(world: &mut World) -> Self {
        Self {
            state: SystemState::new(world),
        }
    }
}

impl SliderSystemState {
    fn run(&mut self, world: &mut World) {
        let (engaged, slider, windows) = self.state.get(world);
        let cursor_pos = windows
            .get_primary()
            .and_then(|window| window.cursor_position());
        if let (Ok(engaged), Some(cursor_pos)) = (engaged.get_single(), cursor_pos) {
            let (node, pos) = slider.get(engaged.slider_entity).unwrap();
            let len = node.size.x;
            let start = pos.translation.x - len / 2.;
            let current = cursor_pos.x - engaged.initial_offset.x;
            let percent = (current - start) / len;
            let gp = engaged.get_percent.clone();
            let p = gp(world);
            *p = percent;
        }
    }

    fn system(world: &mut World) {
        world.resource_scope(|w, mut x: Mut<Self>| {
            x.run(w);
        })
    }
}
