use bevy::{prelude::*, PipelinedDefaultPlugins};
use derive_more::{Deref, DerefMut};
use std::ops::Deref;
use ui4::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(PipelinedDefaultPlugins)
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root));

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

    ctx.with(State(0))
        .with(List::default())
        .with(EditedText("".to_string()))
        .children(|ctx: &mut McCtx| {
            ctx.c(|ctx| text("Hello!")(ctx).with(Height(Units::Pixels(30.))))
                .c(|ctx| text("How are you doing?")(ctx).with(Height(Units::Pixels(30.))))
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
                .c(|ctx| {
                    text(state.map(|s: &State| format!("The number is {}", s.0)))(ctx)
                        .with(Height(Units::Pixels(30.)))
                })
                .c(|ctx| {
                    textbox(edited_text.lens(EditedText::F0))(ctx).with(Height(Units::Pixels(30.)))
                });
        })
        .child(|ctx: Ctx| {
            ctx.with(LayoutType::Row)
                .with(Height(Units::Pixels(30.)))
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
                            ctx.c(|ctx| {
                                text("Now you see me".to_string())(ctx)
                                    .with(Height(Units::Pixels(30.)))
                            });
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
        ctx.with(LayoutType::Row)
            .with(State(0))
            .with(Height(Units::Pixels(30.)))
            .children(move |ctx: &mut McCtx| {
                ctx.c(text(label))
                    .c(|ctx| {
                        button("+".to_string())(ctx)
                            .with(Width(Units::Pixels(50.)))
                            .with(Height(Units::Pixels(30.)))
                            .with(OnClick::new(move |w| {
                                w.get_mut::<State>(entity).unwrap().0 += 1;
                            }))
                    })
                    .c(|ctx| {
                        text(component.map(|x: &State| x.0.to_string()))(ctx)
                            .with(Width(Units::Pixels(50.)))
                            .with(Height(Units::Pixels(30.)))
                    })
                    .c(|ctx| {
                        button("-".to_string())(ctx)
                            .with(Width(Units::Pixels(50.)))
                            .with(Height(Units::Pixels(30.)))
                            .with(OnClick::new(move |w| {
                                w.get_mut::<State>(entity).unwrap().0 -= 1;
                            }))
                    });
            })
    }
}
