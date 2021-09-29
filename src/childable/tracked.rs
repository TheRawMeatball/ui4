mod vec;

pub use vec::{TrackedItemObserver, TrackedVec};

use crate::runtime::UpdateFunc;

trait TrackedObserverExt: Sized {
    fn for_each<F, Ff>(self, f: F) -> TrackedForeach<Self, F>;
}

#[derive(Clone)]
enum Diff<T> {
    Init(Vec<T>),
    Push(T),
    Pop,
    Replace(T, usize),
    // To be supported when Children supports it
    // Switch(usize, usize),
    Remove(usize),
    Insert(T, usize),
    Clear,
}

pub struct TrackedMarker;

struct TrackedForeach<UO, F>(UO, F);

type TrackedAnyList<T> = Vec<(T, Vec<UpdateFunc>)>;
