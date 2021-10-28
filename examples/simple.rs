use bevy::prelude::*;
use derive_more::{Deref, DerefMut};
use std::ops::Deref;
use ui4::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root))
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default());

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component)]
    struct State(i32);

    #[derive(Component, Default, DerefMut, Deref)]
    struct List(TrackedVec<String>);

    #[derive(Component, Default, DerefMut, Deref, Lens)]
    struct EditedText(String);

    let state = ctx.component();
    let list = ctx.component::<List>();
    let edited_text = ctx.component();
    let this = ctx.current_entity();

    ctx.with_bundle(NodeBundle::default())
        .with(Style {
            size: Size {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
            },
            flex_direction: FlexDirection::ColumnReverse,
            ..Default::default()
        })
        .with(State(0))
        .with(List::default())
        .with(EditedText("".to_string()))
        .children(|ctx: &mut McCtx| {
            ctx.c(text("Hello!"))
                .c(text("How are you doing?"))
                .c(|ctx| {
                    button("Increment")(ctx).with(OnClick::new(move |world| {
                        world.get_mut::<State>(this).unwrap().0 += 1;
                    }))
                })
                .c(|ctx| {
                    button("Decrement")(ctx).with(OnClick::new(move |world| {
                        world.get_mut::<State>(this).unwrap().0 -= 1;
                    }))
                })
                .c(text(
                    state.map(|s: &State| format!("The number is {}", s.0)),
                ))
                .c(textbox(edited_text.lens(EditedText::F0)));
        })
        .child(|ctx: Ctx| {
            ctx.with_bundle(NodeBundle::default())
                .child(|ctx| {
                    button("Add Hello".to_string())(ctx).with(OnClick::new(move |w| {
                        w.get_mut::<List>(this).unwrap().push("Hello".to_string());
                    }))
                })
                .child(|ctx| {
                    button("Add Hoi".to_string())(ctx).with(OnClick::new(move |w| {
                        w.get_mut::<List>(this).unwrap().push("Hoi".to_string());
                    }))
                })
                .child(|ctx| {
                    button("Remove last".to_string())(ctx).with(OnClick::new(move |w| {
                        w.get_mut::<List>(this).unwrap().pop();
                    }))
                })
                .child(|ctx| {
                    button("Remove first".to_string())(ctx).with(OnClick::new(move |w| {
                        w.get_mut::<List>(this).unwrap().remove(0);
                    }))
                })
        })
        .children(
            list.map(Deref::deref)
                .each(|label: TrackedItemObserver<String>| {
                    move |ctx: &mut McCtx| {
                        ctx.c(counter(label.map(|s: (&String, usize)| s.0.clone())));
                    }
                }),
        )
        .children(
            res()
                .map(|time: &Time| time.seconds_since_startup() as usize % 2 == 0)
                .map_child(|b| {
                    move |ctx: &mut McCtx| {
                        if b {
                            ctx.c(text("Now you see me".to_string()));
                        }
                    }
                }),
        )
}

fn counter<M>(label: impl IntoObserver<String, M>) -> impl FnOnce(Ctx) -> Ctx {
    #[derive(Component)]
    struct State(i32);

    move |ctx: Ctx| {
        let component = ctx.component();
        let entity = ctx.current_entity();
        ctx.with_bundle(NodeBundle::default())
            .with(Style {
                align_self: AlignSelf::FlexStart,
                ..Default::default()
            })
            .with(State(0))
            .children(move |ctx: &mut McCtx| {
                ctx.c(text(label))
                    .c(|ctx| {
                        button("+".to_string())(ctx).with(OnClick::new(move |w| {
                            w.get_mut::<State>(entity).unwrap().0 += 1;
                        }))
                    })
                    .c(|ctx| {
                        text(component.map(|x: &State| x.0.to_string()))(ctx).with(Style {
                            align_self: AlignSelf::FlexStart,
                            min_size: Size {
                                width: Val::Px(50.0),
                                height: Val::Undefined,
                            },
                            max_size: Size {
                                width: Val::Undefined,
                                height: Val::Px(30.),
                            },
                            ..Default::default()
                        })
                    })
                    .c(|ctx| {
                        button("-".to_string())(ctx).with(OnClick::new(move |w| {
                            w.get_mut::<State>(entity).unwrap().0 -= 1;
                        }))
                    });
            })
    }
}
