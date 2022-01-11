//! # The ui4 getting started guide
//!
//! ## Widgets
//!
//! In ui4, a widget is any function or closure with the signature `fn(Ctx) -> Ctx`.
//! An example widget:
//! ```
//! # use ui4::prelude::*;
//! fn root(ctx: Ctx) -> Ctx {
//!     ctx
//! }
//! ```
//!
//! This is a simple widget which takes no inputs and does nothing. Luckily, this sort of
//! simplicity is just what we need when getting started. We can add this widget as a root widget
//! by using a [`Ui4Root`](crate::prelude::Ui4Root) plugin:
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! # fn root(ctx: Ctx) -> Ctx {ctx}
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugin(Ui4Plugin)
//!         .add_plugin(Ui4Root(root));
//! }
//! ```
//! Congratulations! You've now successfully added ui4 to your game. Now, make it do stuff!
//!
//! ## `Ctx`
//!
//! The [`Ctx`](crate::prelude::Ctx) type is the entry point for ui4. With it, you can add various
//! components to your ui nodes to change their color or layout:
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! # fn root(ctx: Ctx) -> Ctx {
//! ctx.with(UiColor(Color::BLACK))
//!     .with(Width(Units::Pixels(200.)))
//!     .with(Height(Units::Pixels(200.)))
//! # }
//! ```
//! For a comprehensive list of the components you can add to your nodes to change how they look,
//! look at [`ui4::dom`](crate::dom).
//!
//! With [`Ctx`](crate::prelude::Ctx), you can also add children:
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! # fn root(ctx: Ctx) -> Ctx {
//! ctx.child(|ctx: Ctx| {
//!     ctx.with(UiColor(Color::RED))
//! })
//! # }
//! ```
//! The child can be anything that implements `FnOnce(Ctx) -> Ctx`, so you can either declare a child inline, or
//! use one of the predefined widgets such as a button:
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! # fn root(ctx: Ctx) -> Ctx {
//! ctx.child(button("Hi!"))
//! # }
//! ```
//! And now that we have a button, we can define it's behavior by adding more components to it:
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! # fn root(ctx: Ctx) -> Ctx {
//! ctx.child(button("Hi!").with(OnClick::new(|world| println!("You clicked the button!"))))
//! # }
//! ```
//!
//! <details>
//! <summary>Wondering where the `with` on `button()` s return value came from? Click here!</summary>
//!
//! The answer is that it came from a trait, [`crate::ctx::WidgetBuilderExtWith`] to be exact.
//! This trait is implemented for all types that implement `FnOnce(Ctx) -> Ctx`, so any widget, and the method returns
//! yet another `impl FnOnce(Ctx) -> Ctx`. What this method does is reduce boilerplate by avoiding the need to declare
//! a custom widget function yourself like this:
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! # fn root(ctx: Ctx) -> Ctx {
//! ctx.child(|ctx: Ctx| button("Hi!")(ctx).with(OnClick::new(|world| println!("You clicked the button!"))))
//! # }
//! ```
//!
//! On stable, this is implemented by repeatedly boxing, which is rather inefficient. Luckily, by enabling the nightly
//! feature and opting into using `feature(type_alias_impl_trait)`, this can be implemented with no runtime overhead.
//! </details>
//!
//! ## State
//!
//! Just printing to stdout isn't particularly interesting. Luckily, the function [`OnClick`](crate::prelude::OnClick) has
//! `&mut World`, which means it can do arbitrary changes to state, including the state on the widget itself. Right now,
//! the root widget doesn't have any state, but it can be added:
//!
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! fn root(ctx: Ctx) -> Ctx {
//!     #[derive(Component)]
//!     struct RootWidgetState(u32);
//!     let e = ctx.current_entity();
//!
//!     ctx.with(RootWidgetState(0))
//!         .child(button("Increment Counter").with(OnClick::new(move |world| {
//!             let mut state = world.get_mut::<RootWidgetState>(e).unwrap();
//!             state.0 += 1;
//!             println!("Counter is at {}", state.0);
//!         })))
//! }
//! ```
//! This is better, but the state is still only seen as messages from stdout - luckily, we can fix this through data binding.
//!
//! ## Data Binding
//!
//! ui4 supports both one-way and two way data binding, using observers and lenses respectively. Here, what we want is a one way binding
//! from a `u32` in a component to a string to display, so we will use observers:
//!
//! Note: all lenses are also observers.
//! ```
//! # use bevy::prelude::*;
//! # use ui4::prelude::*;
//! fn root(ctx: Ctx) -> Ctx {
//!     #[derive(Component)]
//!     struct RootWidgetState(u32);
//!     let e = ctx.current_entity();
//!     let button_text = ctx
//!         .component()
//!         .map(|c: &RootWidgetState| format!("Counter is at {}", c.0));
//!
//!     ctx.with(RootWidgetState(0))
//!         .child(button(button_text).with(OnClick::new(move |world| {
//!             let mut state = world.get_mut::<RootWidgetState>(e).unwrap();
//!             state.0 += 1;
//!         })))
//! }
//! ```
//!
//! Observers are very useful, and they can be given both to many built-in widgets as well as `with` calls, letting you control the components on your widgets
//! with reactivity as well.
//!

#[allow(unused)]
use crate::prelude::*;
