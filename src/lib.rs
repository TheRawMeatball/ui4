mod dom;
mod input;
mod insertable;
mod runtime;

pub mod animation;
pub mod childable;
pub mod ctx;
pub mod lens;
pub mod observer;
pub mod plugin;
pub mod widgets;

pub mod prelude {
    use super::*;
    pub use animation::{TransitionBundle, TransitionProgress, TweenExt};
    pub use childable::{
        tracked::{TrackedItemObserver, TrackedMarker, TrackedObserverExt, TrackedVec},
        ChildMapExt, Childable,
    };
    pub use ctx::{Ctx, McCtx};
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

pub struct Static;
pub struct Dynamic;
