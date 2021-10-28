use bevy::prelude::*;
use derive_more::{Deref, DerefMut};
use std::{ops::Deref, sync::Arc};
use ui4::prelude::*;

#[derive(Default, Deref, Lens)]
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
        .init_resource::<TodoList>();

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
        .with(UiColor(Color::BLACK))
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
                .child(|ctx| text("Todos")(ctx).with(TextSize(80.)))
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
                // .with(res().map(|assets: &UiAssets| assets.list_background.clone()))
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
                        .child(|ctx: Ctx| {
                            textbox(res().lens(EditedText::F0))(ctx).with(Style {
                                size: Size::new(Val::Percent(90.), Val::Px(30.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                ..Default::default()
                            })
                        })
                        .child(|ctx: Ctx| {
                            button("Add")(ctx)
                                .with(OnClick::new(|world: &mut World| {
                                    let text = std::mem::take(
                                        &mut world.get_resource_mut::<EditedText>().unwrap().0,
                                    );
                                    world.get_resource_mut::<TodoList>().unwrap().push(Todo {
                                        text: text.into(),
                                        done: false,
                                    });
                                }))
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
                item.map(|t: (&Todo, usize)| t.0.done)
                    .dedup()
                    .map(|&done: &bool| if done { Color::GREEN } else { Color::NONE })
                    .map(UiColor),
            )
            .child(|ctx| {
                text(item.map(|t: (&Todo, usize)| t.0.text.to_string()))(ctx).with(TextSize(32.))
            })
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
                                    .child(|ctx: Ctx| {
                                        button("Unmark")(ctx)
                                            .with(Style {
                                                size: Size::new(Val::Percent(50.), Val::Px(30.0)),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..Default::default()
                                            })
                                            .with(item.map(|(_, i): (&Todo, usize)| i).dedup().map(
                                                |&i: &usize| {
                                                    OnClick::new(move |world| {
                                                        let mut list = world
                                                            .get_resource_mut::<TodoList>()
                                                            .unwrap();
                                                        let text = list[i].text.clone();
                                                        list.replace(Todo { text, done: false }, i);
                                                    })
                                                },
                                            ))
                                    })
                                    .child(|ctx: Ctx| {
                                        button("Remove")(ctx)
                                            .with(Style {
                                                size: Size::new(Val::Percent(50.), Val::Px(30.0)),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..Default::default()
                                            })
                                            .with(item.map(|(_, i): (&Todo, usize)| i).dedup().map(
                                                |&i: &usize| {
                                                    OnClick::new(move |world| {
                                                        let mut list = world
                                                            .get_resource_mut::<TodoList>()
                                                            .unwrap();
                                                        list.remove(i);
                                                    })
                                                },
                                            ))
                                    })
                            });
                        } else {
                            ctx.c(|ctx: Ctx| {
                                button("Mark Complete")(ctx)
                                    .with(Style {
                                        size: Size::new(Val::Px(250.), Val::Px(30.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,

                                        ..Default::default()
                                    })
                                    .with(item.map(|(_, i): (&Todo, usize)| i).dedup().map(
                                        |&i: &usize| {
                                            OnClick::new(move |world| {
                                                let mut list =
                                                    world.get_resource_mut::<TodoList>().unwrap();
                                                let text = list[i].text.clone();
                                                list.replace(Todo { text, done: true }, i);
                                            })
                                        },
                                    ))
                            });
                        }
                    }
                },
            ))
    }
}
