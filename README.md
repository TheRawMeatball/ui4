# ui4

ui4 is my fourth major attempt at making a UI dataflow library for the [Bevy](https://github.com/bevyengine/bevy) game engine. More specifically, it's a vdom-less UI library which uses fine-grained reactivity to keep your UI in sync with the rest of your game and itself.

## Warning

This library is *incredibly* young and untested. Try at your own risk!

## Why ui4

UI in bevy, as of 0.5, can get incredibly boilerplate-y. So can code made with this lib, but (hopefully) you'll find the same amount of boilerplate gets you much further with this crate :)

More specifically, this lib offers a widget abstraction, reactivity, and a larger collection of built-in widgets! For now, just copy-paste the widget you need from one of the examples if it exists.

## Usage

For how to use this lib, look at the [examples](examples) folder!

Important note: You'll need nightly rust, and use [a custom branch of bevy](https://github.com/TheRawMeatball/bevy/tree/runs-ui4), as there's still multiple PR s this library needs that haven't been merged into main yet.

## Help

For help with using this lib, feel free to talk to @TheRawMeatball#9628 on the [bevy discord](https://discord.gg/bevy). I'm pretty active, so if you have questions ask away! And if you find a bug, a github issue would be appreciated :)
