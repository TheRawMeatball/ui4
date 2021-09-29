use std::{
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    panic::Location,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use bevy::{ecs::prelude::*, utils::HashSet};

use crate::button::ButtonSystemState;

#[derive(Default)]
pub(crate) struct UiScratchSpace {
    update_hashset_a: HashSet<UpdateFunc>,
    update_hashset_b: HashSet<UpdateFunc>,
}

impl UiScratchSpace {
    pub fn register_update_func(&mut self, uf: UpdateFunc) {
        self.update_hashset_a.insert(uf);
    }

    pub fn register_update_funcs(&mut self, ufs: impl IntoIterator<Item = UpdateFunc>) {
        self.update_hashset_a.extend(ufs);
    }
}

pub(crate) struct UiManagedSystems(pub(crate) SystemStage);

pub(crate) fn primary_ui_system(world: &mut World) {
    world.resource_scope(|world, mut buttons: Mut<ButtonSystemState>| {
        buttons.run(world);
    });
    world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
        systems.0.run(world);
    });
    loop {
        let ui = &mut *world.get_resource_mut::<UiScratchSpace>().unwrap();
        std::mem::swap(&mut ui.update_hashset_a, &mut ui.update_hashset_b);
        let mut update_hashset = std::mem::take(&mut ui.update_hashset_b);

        for uf in update_hashset.drain() {
            uf.run(world);
        }

        let mut ui = &mut *world.get_resource_mut::<UiScratchSpace>().unwrap();
        ui.update_hashset_b = update_hashset;
        if ui.update_hashset_a.is_empty() {
            break;
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpdateFunc(Arc<UfInner<dyn FnMut(&mut World) + Send + Sync>>);
struct UfInner<F: ?Sized> {
    flag: AtomicBool,
    created_at: &'static Location<'static>,
    func: Mutex<F>,
}

impl<F: ?Sized> Debug for UfInner<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UfInner")
            .field("created_at", &self.created_at)
            .finish()
    }
}

#[derive(Component)]
pub(crate) struct UfMarker<T>(
    Arc<UfInner<dyn FnMut(&mut World) + Send + Sync>>,
    PhantomData<T>,
);

impl<T> Drop for UfMarker<T> {
    fn drop(&mut self) {
        self.0
            .flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl UpdateFunc {
    #[track_caller]
    pub(crate) fn new<T, F: FnMut(&mut World) + Send + Sync + 'static>(
        func: F,
    ) -> (Self, UfMarker<T>) {
        let arc = Arc::new(UfInner {
            flag: AtomicBool::new(false),
            created_at: std::panic::Location::caller(),
            func: Mutex::new(func),
        });
        (Self(arc.clone()), UfMarker(arc, PhantomData))
    }
    pub fn run(&self, world: &mut World) {
        if !self.flagged() {
            (self.0.func.lock().unwrap())(world);
        }
    }

    pub fn flagged(&self) -> bool {
        self.0.flag.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Eq for UpdateFunc {}
impl PartialEq for UpdateFunc {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}
impl Hash for UpdateFunc {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}
impl PartialOrd for UpdateFunc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Arc::as_ptr(&self.0).partial_cmp(&Arc::as_ptr(&other.0))
    }
}
impl Ord for UpdateFunc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Arc::as_ptr(&self.0).cmp(&Arc::as_ptr(&other.0))
    }
}
