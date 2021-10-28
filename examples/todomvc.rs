use bevy::prelude::*;
use derive_more::{Deref, DerefMut};
use std::{borrow::Borrow, ops::Deref, sync::Arc};
use ui4::prelude::Text;
use ui4::prelude::*;

struct UiAssets {
    background: Handle<ColorMaterial>,
    button: Handle<ColorMaterial>,
    button_hover: Handle<ColorMaterial>,
    button_click: Handle<ColorMaterial>,
    list_background: Handle<ColorMaterial>,
    text_style: TextStyle,
    transparent: Handle<ColorMaterial>,
    done: Handle<ColorMaterial>,
}

fn init_system(
    mut commands: Commands,
    mut assets: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(UiAssets {
        background: assets.add(Color::BLACK.into()),
        list_background: assets.add(Color::NAVY.into()),
        transparent: assets.add(Color::NONE.into()),
        done: assets.add(Color::DARK_GREEN.into()),
        button: assets.add(Color::DARK_GRAY.into()),
        button_hover: assets.add(Color::GRAY.into()),
        button_click: assets.add(Color::SILVER.into()),
        text_style: TextStyle {
            color: Color::WHITE,
            font: asset_server.load("FiraMono-Medium.ttf"),
            font_size: 32.0,
        },
    })
}

#[derive(Default, Deref)]
struct EditedText(String);

#[derive(Default, Deref, DerefMut)]
struct TodoList(TrackedVec<Todo>);

#[derive(Clone)]
struct Todo {
    text: Arc<str>,
    done: bool,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root))
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .init_resource::<EditedText>()
        .init_resource::<TodoList>()
        .add_startup_system(init_system);

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
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
        .child(|ctx: Ctx| {
            ctx.with_bundle(NodeBundle::default())
                .with(Style {
                    size: Size {
                        width: Val::Auto,
                        height: Val::Undefined,
                    },
                    flex_direction: FlexDirection::ColumnReverse,
                    align_self: AlignSelf::Center,
                    ..Default::default()
                })
                .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
                .child(text(80., "Todos"))
        })
        .child(|ctx: Ctx| {
            ctx.with_bundle(NodeBundle::default())
                .with(Style {
                    size: Size {
                        width: Val::Percent(60.),
                        height: Val::Auto,
                    },
                    flex_direction: FlexDirection::ColumnReverse,
                    align_self: AlignSelf::Center,
                    ..Default::default()
                })
                .with(res().map(|assets: &UiAssets| assets.list_background.clone()))
                .child(|ctx: Ctx| {
                    ctx.with_bundle(NodeBundle::default())
                        .with(Style {
                            size: Size {
                                width: Val::Percent(100.),
                                height: Val::Auto,
                            },
                            flex_direction: FlexDirection::Row,
                            ..Default::default()
                        })
                        .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
                        .child(|ctx: Ctx| {
                            textbox(res().map(|t: &EditedText| t.0.clone()), |w| {
                                &mut w.get_resource_mut::<EditedText>().unwrap().into_inner().0
                            })(ctx)
                            .with(Style {
                                size: Size::new(Val::Percent(90.), Val::Px(30.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                ..Default::default()
                            })
                        })
                        .child(|ctx: Ctx| {
                            button("Add", |world: &mut World| {
                                let text = std::mem::take(
                                    &mut world.get_resource_mut::<EditedText>().unwrap().0,
                                );
                                world.get_resource_mut::<TodoList>().unwrap().push(Todo {
                                    text: text.into(),
                                    done: false,
                                });
                            })(ctx)
                            .with(Style {
                                size: Size::new(Val::Percent(10.), Val::Px(30.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..Default::default()
                            })
                        })
                })
                .children(res::<TodoList>().map(Deref::deref).each(
                    |item: TrackedItemObserver<Todo>| {
                        move |ctx: &mut McCtx| {
                            ctx.c(todo(item));
                        }
                    },
                ))
        })
}

fn todo(item: TrackedItemObserver<Todo>) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with_bundle(NodeBundle::default())
            .with(Style {
                size: Size {
                    width: Val::Percent(100.),
                    height: Val::Auto,
                },
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            })
            .with(
                res()
                    .and(item.map(|t: (&Todo, usize)| t.0.done).dedup())
                    .map(|(assets, &done): (&UiAssets, &bool)| {
                        if done {
                            assets.done.clone()
                        } else {
                            assets.transparent.clone()
                        }
                    }),
            )
            .child(text(
                32.,
                item.map(|t: (&Todo, usize)| t.0.text.to_string()),
            ))
            .children(item.map(|(todo, _): (&Todo, usize)| todo.done).map_child(
                move |done: bool| {
                    move |ctx: &mut McCtx| {
                        if done {
                            ctx.c(|ctx: Ctx| {
                                ctx.with_bundle(NodeBundle::default())
                                    .with(Style {
                                        size: Size::new(Val::Px(250.), Val::Px(30.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..Default::default()
                                    })
                                    .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
                                    .child(|ctx: Ctx| {
                                        button("Unmark", |_| {})(ctx)
                                            .with(Style {
                                                size: Size::new(Val::Percent(50.), Val::Px(30.0)),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..Default::default()
                                            })
                                            .with(item.map(|(_, i): (&Todo, usize)| i).dedup().map(
                                                |&i: &usize| {
                                                    ClickFunc(ButtonFunc::new(move |world| {
                                                        let mut list = world
                                                            .get_resource_mut::<TodoList>()
                                                            .unwrap();
                                                        let text = list[i].text.clone();
                                                        list.replace(Todo { text, done: false }, i);
                                                    }))
                                                },
                                            ))
                                    })
                                    .child(|ctx: Ctx| {
                                        button("Remove", |_| {})(ctx)
                                            .with(Style {
                                                size: Size::new(Val::Percent(50.), Val::Px(30.0)),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..Default::default()
                                            })
                                            .with(item.map(|(_, i): (&Todo, usize)| i).dedup().map(
                                                |&i: &usize| {
                                                    ClickFunc(ButtonFunc::new(move |world| {
                                                        let mut list = world
                                                            .get_resource_mut::<TodoList>()
                                                            .unwrap();
                                                        list.remove(i);
                                                    }))
                                                },
                                            ))
                                    })
                            });
                        } else {
                            ctx.c(|ctx: Ctx| {
                                button("Mark Complete", |_| {})(ctx)
                                    .with(Style {
                                        size: Size::new(Val::Px(250.), Val::Px(30.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,

                                        ..Default::default()
                                    })
                                    .with(item.map(|(_, i): (&Todo, usize)| i).dedup().map(
                                        |&i: &usize| {
                                            ClickFunc(ButtonFunc::new(move |world| {
                                                let mut list =
                                                    world.get_resource_mut::<TodoList>().unwrap();
                                                let text = list[i].text.clone();
                                                list.replace(Todo { text, done: true }, i);
                                            }))
                                        },
                                    ))
                            });
                        }
                    }
                },
            ))
    }
}
