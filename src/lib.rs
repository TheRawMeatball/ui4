#![cfg_attr(feature = "nightly", feature(type_alias_impl_trait))]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

pub mod animation;
pub mod ctx;
pub mod dom;
pub mod plugin;
pub mod widgets;

mod childable;
mod input;
mod insertable;
mod observer;
mod runtime;

#[doc(hidden)]
pub mod lens;

pub mod prelude {
    use super::*;
    pub use animation::{TransitionBundle, TransitionProgress, TweenExt};
    pub use childable::{
        tracked::{TrackedItemObserver, TrackedMap, TrackedMarker, TrackedObserverExt, TrackedVec},
        ChildMapExt, Childable,
    };
    pub use ctx::{Ctx, McCtx, WidgetBuilderExtWith, WidgetBuilderExtWithModified};
    pub use dom::layout::{layout_components::*, Units};
    pub use dom::{Color as UiColor, Focused, HideOverflow, Text, TextDetails, TextFont, TextSize};
    pub use lens::WorldLens;
    pub use observer::{res, single, IntoObserver, ObserverExt};
    pub use plugin::{Ui4Plugin, Ui4Root};
    pub use widgets::button::{OnClick, OnHover, OnRelease, OnUnhover};
    pub type ObsReturn<'a, T, M, O> =
        <<O as IntoObserver<T, M>>::ReturnSpec as observer::ReturnSpec<'a, T>>::R;

    pub use widgets::{
        button, checkbox, dropdown, progressbar, radio_button, slider, text, text_fade, textbox,
    };

    pub use std::borrow::Borrow;

    pub use bevy::render2::color::Color;

    pub use ui4_macros::Lens;
}

#[doc(hidden)]
pub struct Static;
#[doc(hidden)]
pub struct Dynamic;
#[doc(hidden)]
pub struct OptionalDynamic;
