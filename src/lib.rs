mod dom;
mod insertable;
mod runtime;

pub mod animation;
pub mod button;
pub mod childable;
pub mod ctx;
pub mod lens;
pub mod observer;
pub mod plugin;
pub mod widgets;

pub mod prelude {
    use super::*;
    pub use animation::{TransitionBundle, TransitionProgress, TweenExt};
    pub use button::{ButtonFunc, ClickFunc, FuncScratch, HoverFunc, ReleaseFunc, UnhoverFunc};
    pub use childable::{
        tracked::{TrackedItemObserver, TrackedMarker, TrackedObserverExt, TrackedVec},
        ChildMapExt, Childable,
    };
    pub use ctx::{Ctx, McCtx};
    pub use dom::layout::layout_components::*;
    pub use dom::{Color as UiColor, Text};
    pub use lens::WorldLens;
    pub use morphorm::Units;
    pub use observer::{res, single, IntoObserver, ObserverExt};
    pub use plugin::{Ui4Plugin, Ui4Root};
    pub type ObsReturn<'a, T, M, O> =
        <<O as IntoObserver<T, M>>::ReturnSpec as observer::ReturnSpec<'a, T>>::R;

    pub use widgets::{
        button, checkbox, dropdown, progressbar, radio_button, slider, text, text_fade, textbox,
    };

    pub use std::borrow::Borrow;

    pub use ui4_macros::Lens;
}

pub struct Static;
pub struct Dynamic;
