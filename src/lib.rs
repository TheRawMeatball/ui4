#![feature(generic_associated_types)]
#![feature(unboxed_closures)]

mod insertable;
mod runtime;

pub mod button;
pub mod childable;
pub mod ctx;
pub mod observer;
pub mod plugin;

pub mod prelude {
    use super::*;
    pub use button::{ButtonFunc, ClickFunc, FuncScratch, HoverFunc, ReleaseFunc, UnhoverFunc};
    pub use childable::{tracked::TrackedVec, Childable};
    pub use ctx::{Ctx, McCtx};
    pub use observer::{res, IntoObserver, ObserverExt};
    pub use plugin::Ui4Plugin;
}

pub struct Static;
pub struct Dynamic;
