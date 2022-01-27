# ui4

ui4 is my fourth major attempt at making a UI dataflow library for the [Bevy](https://github.com/bevyengine/bevy) game engine. More specifically, it's a vdom-less UI library which uses fine-grained reactivity to keep your UI in sync with the rest of your game and itself.

## Warning

This library is *incredibly* young and untested. Try at your own risk!

## Why ui4

UI in bevy, as of 0.6, can get incredibly boilerplate-y. So can code made with this lib, but (hopefully) you'll find the same amount of boilerplate gets you much further with this crate :)

More specifically, this lib offers a widget abstraction, reactivity, animations, and a collection of built-in widgets!

## Usage

```rust
use bevy::{prelude::*, PipelinedDefaultPlugins};
use ui4::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(PipelinedDefaultPlugins)
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root));

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component)]
    struct State(i32);

    let state = ctx.component();
    let this = ctx.current_entity();

    ctx.with(State(0))
        .with(Top(Units::Pixels(50.)))
        .with(Left(Units::Pixels(50.)))
        .child(text("Hello!").with(Height(Units::Pixels(30.))))
        .child(|ctx| {
            ctx.with(Width(Units::Pixels(300.)))
                .with(Height(Units::Pixels(30.)))
                .with(LayoutType::Row)
                .child(button("Increment").with(OnClick::new(move |world| {
                    world.get_mut::<State>(this).unwrap().0 += 1;
                })))
                .child(button("Decrement").with(OnClick::new(move |world| {
                    world.get_mut::<State>(this).unwrap().0 -= 1;
                })))
                .child(text(
                    state.map(|s: &State| format!("The number is {}", s.0)),
                ))
        })
}
```

For more examples on how to use this library, look at the [examples](examples) folder, and also consider looking at the [tutorial module on docs.rs](https://docs.rs/ui4/latest/ui4/tutorial/index.html).

Important note: This crate works around certain limitations of stable rust using boxing, so switching to nightly and enabling the `nightly` feature might improve performance, and is recommended.

## Help

For help with using this lib, feel free to talk to @TheRawMeatball#9628 on [the bevy discord](https://discord.gg/bevy). I'm pretty active, so if you have questions ask away! And if you find a bug, a github issue would be appreciated :)
