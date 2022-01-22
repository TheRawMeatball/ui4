//! `ui4` a UI library for the [Bevy](https://github.com/bevyengine/bevy) game engine.
//! More specifically, it's a vdom-less UI library which uses fine-grained reactivity to keep your UI in sync with the rest of your game and itself.

#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

/// Types and traits for the transition system
pub mod animation;
/// The core api that makes ui4 tick
pub mod ctx;
/// Types that make up the DOM
pub mod dom;
/// The bevy integration
pub mod plugin;
/// The built-in widget library
pub mod widgets;

pub mod childable;
pub mod insertable;
pub mod lens;
pub mod observer;

mod input;
mod runtime;

/// A comprehensive getting started guide for ui4
pub mod tutorial;

/// The ui4 prelude
pub mod prelude {
    use super::*;
    pub use animation::{TransitionBundle, TransitionProgress, TweenExt};
    pub use childable::{
        tracked::{IndexObserver, TrackedItemLens, TrackedMarker, TrackedObserverExt, TrackedVec},
        ChildMapExt, Childable,
    };
    pub use ctx::{Ctx, McCtx, WidgetBuilderExtWith, WidgetBuilderExtWithModified};
    pub use dom::layout::{layout_components::*, Units};
    pub use dom::{Focused, HideOverflow, TextAlign, TextDetails, TextSize};
    pub use lens::WorldLens;
    pub use observer::{component, res, single, FlattenReturn, IntoObserver, ObserverExt};
    pub use plugin::{Ui4Plugin, Ui4Root};
    pub use widgets::button::{OnClick, OnHover, OnRelease, OnUnhover};
    pub type ObsReturn<'a, T, M, O> =
        <<O as IntoObserver<T, M>>::ReturnSpec as observer::ReturnSpec<'a, T>>::R;

    pub use widgets::{
        button, checkbox, draggable_window, dropdown, progressbar, radio_button, slider, text,
        text_fade, textbox,
    };

    pub use std::borrow::Borrow;

    pub use ui4_macros::Lens;
}

#[doc(hidden)]
pub struct Static;
#[doc(hidden)]
pub struct Dynamic;
#[doc(hidden)]
pub struct OptionalDynamic;
