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
        //.add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_startup_system(init_system);

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component, Deref, DerefMut, Default)]
    struct TextboxText(String);

    #[derive(Component, Deref, DerefMut, Default)]
    struct CheckboxData(bool);

    #[derive(Component, Hash, Copy, Clone, PartialEq, Eq)]
    enum RadioButtonSelect {
        A,
        B,
        C,
    }

    #[derive(Component)]
    struct Slider(f32);

    let this = ctx.current_entity();
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
                textbox(textbox_text.map(|t: &TextboxText| t.0.clone()), move |w| {
                    w.get_mut::<TextboxText>(this).unwrap().into_inner()
                }),
            ))
            .c(labelled_widget(
                "Checkbox",
                checkbox(
                    checkbox_data
                        .map(|t: &CheckboxData| t.0)
                        .dedup()
                        .map(|&b: &bool| b),
                    move |w| w.get_mut::<CheckboxData>(this).unwrap().into_inner(),
                ),
            ))
            .dyn_group(
                checkbox_data
                    .map(|t: &CheckboxData| t.0)
                    .map_child(|b| {
                        move |ctx: &mut McCtx| {
                            if b {
                                ctx.c(text_fade("Now you see me!"));
                            }
                        }
                    }),
            )
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
                        ctx.c(radio_button(
                            radiobutton.map(|x: &RadioButtonSelect| *x),
                            RadioButtonSelect::A,
                            move |w: &mut World| {
                                w.get_mut::<RadioButtonSelect>(this).unwrap().into_inner()
                            },
                        ))
                        .c(text("A  "))
                        .c(radio_button(
                            radiobutton.map(|x: &RadioButtonSelect| *x),
                            RadioButtonSelect::B,
                            move |w: &mut World| {
                                w.get_mut::<RadioButtonSelect>(this).unwrap().into_inner()
                            },
                        ))
                        .c(text("B  "))
                        .c(radio_button(
                            radiobutton.map(|x: &RadioButtonSelect| *x),
                            RadioButtonSelect::C,
                            move |w: &mut World| {
                                w.get_mut::<RadioButtonSelect>(this).unwrap().into_inner()
                            },
                        ))
                        .c(text("C  "));
                    })
            }))
            .c(labelled_widget(
                "Dropdown",
                dropdown(
                    radiobutton.map(|x: &RadioButtonSelect| *x),
                    [
                        (RadioButtonSelect::A, "A"),
                        (RadioButtonSelect::B, "B"),
                        (RadioButtonSelect::C, "C"),
                    ],
                    move |w: &mut World| w.get_mut::<RadioButtonSelect>(this).unwrap().into_inner(),
                ),
            ))
            .c(labelled_widget(
                "Progress",
                progressbar(slider_percent.map(|f: &Slider| f.0)),
            ))
            .c(labelled_widget(
                "Slider",
                slider(
                    slider_percent.map(|f: &Slider| f.0),
                    move |w: &mut World| &mut w.get_mut::<Slider>(this).unwrap().into_inner().0,
                ),
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
            ));
        })
}

fn labelled_widget(
    label: &'static str,
    widget: impl FnOnce(Ctx) -> Ctx,
) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx| {
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

fn text_slide<O: IntoObserver<String, M>, M>(text: O) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        let transition = ctx.component().map(TransitionProgress::progress);
        ctx.with_bundle(TextBundle::default())
            .with_bundle(TransitionBundle::bidirectional(1.))
            .with(transition.map(|size| Style {
                align_self: AlignSelf::FlexStart,
                size: Size::new(Val::Undefined, Val::Px(size * 32.)),
                ..Default::default()
            }))
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
            .with_bundle(TransitionBundle::bidirectional(1.))
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

fn textbox<M, O: IntoObserver<String, M>>(
    text: O,
    get_text: impl Fn(&mut World) -> &mut String + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx {
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
            .with(TextBoxFunc::new(get_text))
            .with(res().map(|assets: &UiAssets| assets.button.clone()))
            .child(|ctx: Ctx| {
                ctx.with_bundle(TextBundle::default())
                    .with(Style {
                        align_self: AlignSelf::FlexStart,
                        ..Default::default()
                    })
                    .with(
                        res()
                            .and(text.into_observer())
                            .and(has_focused.and(cursor).map(
                                |(focused, cursor): (bool, &TextBox)| focused.then(|| cursor.0),
                            ))
                            .map(
                                move |((assets, text), cursor): (
                                    (&UiAssets, O::ObserverReturn<'_, '_>),
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

fn checkbox<M, O: IntoObserver<bool, M>>(
    is_checked: O,
    get_checked: impl Fn(&mut World) -> &mut bool + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx {
    button(
        is_checked
            .into_observer()
            .map(|b: O::ObserverReturn<'_, '_>| *b.borrow())
            .dedup()
            .map(|b: &bool| if *b { "x" } else { " " })
            .map(|s: &'static str| s.to_string()),
        move |w| {
            let val = get_checked(w);
            *val = !*val;
        },
    )
}

fn radio_button<T, O, M>(
    item: O,
    this: T,
    get_item: impl Fn(&mut World) -> &mut T + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx
where
    T: PartialEq + Clone + Send + Sync + 'static,
    O: IntoObserver<T, M>,
{
    let this1 = this.clone();
    button(
        item.into_observer()
            .map(|t: O::ObserverReturn<'_, '_>| -> T { t.borrow().clone() })
            .dedup()
            .map(move |t: &T| if t == &this1 { "x" } else { " " })
            .map(|s: &'static str| s.to_string()),
        move |w| {
            let val = get_item(w);
            *val = this.clone();
        },
    )
}

fn dropdown<O, M, T, const N: usize>(
    selected: O,
    options: [(T, &'static str); N],
    get_item: impl Fn(&mut World) -> &mut T + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx
where
    T: Eq + Hash + Clone + Send + Sync + 'static,
    O: IntoObserver<T, M>,
{
    let options_map: HashMap<_, _> = options.iter().cloned().collect();
    let options = Arc::new(options);
    let get_item = Arc::new(get_item);

    |ctx| {
        let is_open = ctx.has_component::<Focused>();

        button(
            selected
                .into_observer()
                .map(move |s: O::ObserverReturn<'_, '_>| options_map[s.borrow()].to_string()),
            move |_| {},
        )(ctx)
        .with(Style {
            size: Size::new(Val::Undefined, Val::Px(30.0)),
            ..Default::default()
        })
        .with(Focusable)
        .children(is_open.map_child(move |b: bool| {
            let options = Arc::clone(&options);
            let get_item = Arc::clone(&get_item);
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
                                for (item, display) in &*options {
                                    let display: &'static str = display;
                                    let get_item = Arc::clone(&get_item);
                                    let item = item.clone();
                                    ctx.c(button(display, move |w| {
                                        let m_item = get_item(w);
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

fn slider<O: IntoObserver<f32, M>, M>(
    percent: O,
    get_percent: impl Fn(&mut World) -> &mut f32 + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx {
    |ctx| {
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
                    .with(
                        percent
                            .into_observer()
                            .map(|f: O::ObserverReturn<'_, '_>| *f.borrow())
                            .map(|f: f32| Style {
                                size: Size::new(Val::Percent(f * 100.), Val::Auto),
                                justify_content: JustifyContent::FlexEnd,
                                ..Default::default()
                            }),
                    )
                    .with(res().map(|assets: &UiAssets| assets.button_hover.clone()))
                    .child(|ctx: Ctx| {
                        let interaction = ctx.component();
                        let cursor_entity = ctx.current_entity();
                        let get_percent = Arc::new(get_percent);
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
                                        get_percent: get_percent.clone(),
                                    });
                                }
                            })))
                            .with(ReleaseFunc(ButtonFunc::new(move |w| {
                                println!("HEY!");
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
