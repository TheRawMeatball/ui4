use bevy::prelude::*;
use derive_more::{Deref, DerefMut};
use ui4::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root));

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component)]
    struct State(i32);

    #[derive(Component, Default, DerefMut, Deref, Lens)]
    struct List(TrackedVec<String>);

    #[derive(Component, Default, DerefMut, Deref, Lens)]
    struct EditedText(String);

    let state = ctx.component();
    let list = ctx.component::<List>();
    let edited_text = ctx.component();
    let this = ctx.current_entity();

    ctx.with(State(0))
        .with(UiColor(Color::BLACK))
        .with(List::default())
        .with(EditedText("".to_string()))
        .children(|ctx: &mut McCtx| {
            ctx.c(text("Hello!").with(Height(Units::Pixels(30.))))
                .c(text("How are you doing?").with(Height(Units::Pixels(30.))))
                .c(button("Increment").with(OnClick::new(move |world| {
                    world.get_mut::<State>(this).unwrap().0 += 1;
                })))
                .c(button("Decrement").with(OnClick::new(move |world| {
                    world.get_mut::<State>(this).unwrap().0 -= 1;
                })))
                .c(
                    text(state.map(|s: &State| format!("The number is {}", s.0)))
                        .with(Height(Units::Pixels(30.))),
                )
                .c(textbox(edited_text.lens(EditedText::F0)).with(Height(Units::Pixels(30.))));
        })
        .child(|ctx: Ctx| {
            ctx.with(LayoutType::Row)
                .with(Height(Units::Pixels(30.)))
                .child(button("Add Hello".to_string()).with(OnClick::new(move |w| {
                    w.get_mut::<List>(this).unwrap().push("Hello".to_string());
                })))
                .child(button("Add Hoi".to_string()).with(OnClick::new(move |w| {
                    w.get_mut::<List>(this).unwrap().push("Hoi".to_string());
                })))
                .child(
                    button("Remove last".to_string()).with(OnClick::new(move |w| {
                        w.get_mut::<List>(this).unwrap().pop();
                    })),
                )
                .child(
                    button("Remove first".to_string()).with(OnClick::new(move |w| {
                        let mut list = w.get_mut::<List>(this).unwrap();
                        if !list.is_empty() {
                            list.remove(0);
                        }
                    })),
                )
        })
        .children(
            list.lens(List::F0)
                .each(|label: TrackedItemLens<String, _>, _| {
                    move |ctx: &mut McCtx| {
                        ctx.c(counter(label.cloned()));
                    }
                }),
        )
        .children(
            res()
                .map(|time: &Time| time.seconds_since_startup() as usize % 2 == 0)
                .map_child(|b| {
                    move |ctx: &mut McCtx| {
                        if b {
                            ctx.c(
                                text("Now you see me".to_string()).with(Height(Units::Pixels(30.)))
                            );
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
                    .c(button("+".to_string())
                        .with(Width(Units::Pixels(50.)))
                        .with(Height(Units::Pixels(30.)))
                        .with(OnClick::new(move |w| {
                            w.get_mut::<State>(entity).unwrap().0 += 1;
                        })))
                    .c(text(component.map(|x: &State| x.0.to_string()))
                        .with(Width(Units::Pixels(50.)))
                        .with(Height(Units::Pixels(30.))))
                    .c(button("-".to_string())
                        .with(Width(Units::Pixels(50.)))
                        .with(Height(Units::Pixels(30.)))
                        .with(OnClick::new(move |w| {
                            w.get_mut::<State>(entity).unwrap().0 -= 1;
                        })));
            })
    }
}
