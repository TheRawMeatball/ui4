#![feature(generic_associated_types)]
#![feature(unboxed_closures)]

mod insertable;
mod runtime;

pub mod animation;
pub mod button;
pub mod childable;
pub mod ctx;
pub mod observer;
pub mod plugin;
pub mod textbox;

pub mod prelude {
    use super::*;
    pub use animation::{TransitionBundle, TransitionProgress, TweenExt};
    pub use button::{ButtonFunc, ClickFunc, FuncScratch, HoverFunc, ReleaseFunc, UnhoverFunc};
    pub use childable::{
        tracked::{TrackedItemObserver, TrackedMarker, TrackedObserverExt, TrackedVec},
        ChildMapExt, Childable,
    };
    pub use ctx::{Ctx, McCtx};
    pub use observer::{res, single, IntoObserver, ObserverExt};
    pub use plugin::{Ui4Plugin, Ui4Root};
    pub use textbox::{Focusable, Focused, TextBox, TextBoxFunc};
}

pub struct Static;
pub struct Dynamic;
