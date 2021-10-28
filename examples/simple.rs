use bevy::prelude::*;
use derive_more::{Deref, DerefMut};
use std::ops::Deref;
use ui4::prelude::*;

struct UiAssets {
    background: Handle<ColorMaterial>,
    button: Handle<ColorMaterial>,
    button_hover: Handle<ColorMaterial>,
    button_click: Handle<ColorMaterial>,
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
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_startup_system(init_system);

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component)]
    struct State(i32);

    #[derive(Component, Default, DerefMut, Deref)]
    struct List(TrackedVec<String>);

    #[derive(Component, Default, DerefMut, Deref)]
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
        .with(res().map(|assets: &UiAssets| assets.background.clone()))
        .with(State(0))
        .with(List::default())
        .with(EditedText("".to_string()))
        .children(|ctx: &mut McCtx| {
            ctx.c(text("Hello!"))
                .c(text("How are you doing?"))
                .c(button("Increment", move |world| {
                    world.get_mut::<State>(this).unwrap().0 += 1;
                }))
                .c(button("Decrement", move |world| {
                    world.get_mut::<State>(this).unwrap().0 -= 1;
                }))
                .c(text(
                    state.map(|s: &State| format!("The number is {}", s.0)),
                ))
                .c(textbox(
                    edited_text.map(|t: &EditedText| t.to_string()),
                    move |world| world.get_mut::<EditedText>(this).unwrap().into_inner(),
                ));
        })
        .child(|ctx: Ctx| {
            ctx.with_bundle(NodeBundle::default())
                .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
                .child(button("Add Hello".to_string(), move |w| {
                    w.get_mut::<List>(this).unwrap().push("Hello".to_string());
                }))
                .child(button("Add Hoi".to_string(), move |w| {
                    w.get_mut::<List>(this).unwrap().push("Hoi".to_string());
                }))
                .child(button("Remove last".to_string(), move |w| {
                    w.get_mut::<List>(this).unwrap().pop();
                }))
                .child(button("Remove first".to_string(), move |w| {
                    w.get_mut::<List>(this).unwrap().remove(0);
                }))
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
            .with(res().map(|assets: &UiAssets| assets.transparent.clone()))
            .with(State(0))
            .children(move |ctx: &mut McCtx| {
                ctx.c(text(label))
                    .c(button("+".to_string(), move |w| {
                        w.get_mut::<State>(entity).unwrap().0 += 1;
                    }))
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
                    .c(button("-".to_string(), move |w| {
                        w.get_mut::<State>(entity).unwrap().0 -= 1;
                    }));
            })
    }
}
