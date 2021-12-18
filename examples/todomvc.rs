use bevy::prelude::*;
use derive_more::{Deref, DerefMut};
use std::sync::Arc;
use ui4::prelude::*;

#[derive(Default, Deref, Lens)]
struct EditedText(String);

#[derive(Default, Deref, DerefMut, Lens)]
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
        .init_resource::<EditedText>()
        .init_resource::<TodoList>();

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    ctx.with(UiColor(Color::BLACK))
        .child(|ctx: Ctx| {
            ctx.with(ChildRight(Units::Stretch(1.)))
                .with(ChildLeft(Units::Stretch(1.)))
                .child(text("Todos").with(TextSize(80.)))
        })
        .child(|ctx: Ctx| {
            ctx.with(Right(Units::Stretch(1.)))
                .with(Left(Units::Stretch(1.)))
                .with(Width(Units::Percentage(60.)))
                .child(|ctx: Ctx| {
                    ctx.with(Height(Units::Auto))
                        .with(LayoutType::Row)
                        .child(
                            textbox(res().lens(EditedText::F0))
                                .with(Width(Units::Percentage(90.)))
                                .with(Height(Units::Pixels(30.))),
                        )
                        .child(
                            button("Add")
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
                                .with(Height(Units::Pixels(30.))),
                        )
                })
                .children(res::<TodoList>().lens(TodoList::F0).each(|item, index| {
                    move |ctx: &mut McCtx| {
                        ctx.c(todo(item, index));
                    }
                }))
        })
}

fn todo(item: impl WorldLens<Out = Todo>, index: IndexObserver) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with(Width(Units::Percentage(100.)))
            .with(Height(Units::Pixels(30.)))
            .with(LayoutType::Row)
            .with(ColBetween(Units::Stretch(1.)))
            .with(
                item.map(|t: &Todo| t.done)
                    .dedup()
                    .map(|&done: &bool| if done { Color::GREEN } else { Color::NONE })
                    .map(UiColor),
            )
            .child(text(item.map(|t: &Todo| t.text.to_string())).with(TextSize(32.)))
            .children(
                item.map(|todo: &Todo| todo.done)
                    .map_child(move |done: bool| {
                        move |ctx: &mut McCtx| {
                            if done {
                                ctx.c(|ctx: Ctx| {
                                    ctx.with(Width(Units::Pixels(250.)))
                                        .with(Height(Units::Pixels(30.)))
                                        .with(LayoutType::Row)
                                        .child(
                                            button("Unmark")
                                                .with(Width(Units::Percentage(50.)))
                                                .with(Height(Units::Pixels(30.)))
                                                .with(OnClick::new(move |world| {
                                                    item.get_mut(world).done = false;
                                                })),
                                        )
                                        .child(
                                            button("Remove")
                                                .with(Width(Units::Percentage(50.)))
                                                .with(Height(Units::Pixels(30.)))
                                                .with(index.dedup().map(|&i: &usize| {
                                                    OnClick::new(move |world| {
                                                        let mut list = world
                                                            .get_resource_mut::<TodoList>()
                                                            .unwrap();
                                                        list.remove(i);
                                                    })
                                                })),
                                        )
                                });
                            } else {
                                ctx.c(button("Mark Complete")
                                    .with(Width(Units::Pixels(250.)))
                                    .with(Height(Units::Pixels(30.)))
                                    .with(OnClick::new(move |world| {
                                        item.get_mut(world).done = true;
                                    })));
                            }
                        }
                    }),
            )
    }
}
