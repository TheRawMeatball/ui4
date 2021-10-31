use bevy::{prelude::*, PipelinedDefaultPlugins};
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
    app.add_plugins(PipelinedDefaultPlugins)
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root))
        .init_resource::<EditedText>()
        .init_resource::<TodoList>();

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    ctx.with(Width(Units::Percentage(100.)))
        .with(Height(Units::Percentage(100.)))
        .with(UiColor(Color::BLACK))
        .child(|ctx: Ctx| {
            ctx.with(ChildRight(Units::Stretch(1.)))
                .with(ChildLeft(Units::Stretch(1.)))
                .child(|ctx| text("Todos")(ctx).with(TextSize(80.)))
        })
        .child(|ctx: Ctx| {
            ctx.with(Right(Units::Stretch(1.)))
                .with(Left(Units::Stretch(1.)))
                .with(Width(Units::Percentage(60.)))
                .child(|ctx: Ctx| {
                    ctx.with(Height(Units::Auto))
                        .with(LayoutType::Row)
                        .child(|ctx: Ctx| {
                            textbox(res().lens(EditedText::F0))(ctx)
                                .with(Width(Units::Percentage(90.)))
                                .with(Height(Units::Pixels(30.)))
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
                                .with(Width(Units::Percentage(10.)))
                                .with(Height(Units::Pixels(30.)))
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
        ctx.with(Width(Units::Percentage(100.)))
            .with(Height(Units::Pixels(30.)))
            .with(LayoutType::Row)
            .with(ColBetween(Units::Stretch(1.)))
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
                                ctx.with(Width(Units::Pixels(250.)))
                                    .with(Height(Units::Pixels(30.)))
                                    .with(LayoutType::Row)
                                    .child(|ctx: Ctx| {
                                        button("Unmark")(ctx)
                                            .with(Width(Units::Percentage(50.)))
                                            .with(Height(Units::Pixels(30.)))
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
                                            .with(Width(Units::Percentage(50.)))
                                            .with(Height(Units::Pixels(30.)))
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
                                    .with(Width(Units::Pixels(250.)))
                                    .with(Height(Units::Pixels(30.)))
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
