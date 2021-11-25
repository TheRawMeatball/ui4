use std::{
    hash::Hash,
    marker::PhantomData,
    ptr::addr_of_mut,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use ahash;
use bevy::ecs::prelude::*;
use dashmap::DashSet;

use crate::widgets::{button::ButtonSystemState, textbox::TextBoxSystemState};

#[derive(Default)]
pub(crate) struct UiScratchSpace {
    update_hashset_a: DashSet<UpdateFunc, ahash::RandomState>,
    update_hashset_b: DashSet<UpdateFunc, ahash::RandomState>,
}

impl UiScratchSpace {
    pub fn register_update_func(&self, uf: UpdateFunc) {
        self.update_hashset_a.insert(uf);
    }

    pub fn register_update_funcs(&self, ufs: impl IntoIterator<Item = UpdateFunc>) {
        for uf in ufs {
            self.register_update_func(uf);
        }
    }

    pub fn process_list(&self, list: &mut Vec<UpdateFunc>) {
        list.retain(|uf| {
            let flagged = uf.flagged();
            if !flagged {
                self.register_update_func(uf.clone());
            }
            !flagged
        });
    }
}

// Contains internal change detection systems
pub(crate) struct UiManagedSystems(pub(crate) SystemStage);

pub(crate) fn primary_ui_system(world: &mut World) {
    world.resource_scope(|world, mut buttons: Mut<ButtonSystemState>| {
        buttons.run(world);
    });
    world.resource_scope(|world, mut textbox: Mut<TextBoxSystemState>| {
        textbox.run(world);
    });
    world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
        systems.0.run(world);
    });
    loop {
        let ui = &mut *world.get_resource_mut::<UiScratchSpace>().unwrap();
        std::mem::swap(&mut ui.update_hashset_a, &mut ui.update_hashset_b);
        let update_hashset = std::mem::take(&mut ui.update_hashset_b);

        for uf in update_hashset.iter() {
            uf.run(world);
        }
        update_hashset.clear();

        let ui = &mut *world.get_resource_mut::<UiScratchSpace>().unwrap();
        ui.update_hashset_b = update_hashset;
        if ui.update_hashset_a.is_empty() {
            break;
        }
    }
}

#[derive(Clone)]
pub struct UpdateFunc(Arc<UfInner<dyn FnMut(&mut World) + Send + Sync>>);
struct UfInner<F: ?Sized> {
    flag: AtomicBool,
    func: Mutex<F>,
}

#[derive(Component)]
pub(crate) struct UfMarker<T> {
    arc: Arc<UfInner<dyn FnMut(&mut World) + Send + Sync>>,
    list: Vec<UpdateFunc>,
    _marker: PhantomData<T>,
}

impl<T> UfMarker<T> {
    pub fn forget(self) {
        use std::mem::{ManuallyDrop, MaybeUninit};

        let mut md = ManuallyDrop::new(MaybeUninit::new(self));
        unsafe {
            std::ptr::read(addr_of_mut!((*md.as_mut_ptr()).arc));
            std::ptr::read(addr_of_mut!((*md.as_mut_ptr()).list));
        }
    }
}

impl<T> Drop for UfMarker<T> {
    fn drop(&mut self) {
        self.arc
            .flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl<T> UfMarker<T> {
    pub fn add_dependent(&mut self, uf: UpdateFunc) {
        self.list.push(uf);
    }

    pub fn trigger(&mut self, ctx: &mut UiScratchSpace) {
        ctx.process_list(&mut self.list);
    }
}

impl UpdateFunc {
    pub(crate) fn new<T, F: FnMut(&mut World) + Send + Sync + 'static>(
        func: F,
    ) -> (Self, UfMarker<T>) {
        let arc = Arc::new(UfInner {
            flag: AtomicBool::new(false),
            func: Mutex::new(func),
        });
        (
            Self(arc.clone()),
            UfMarker {
                arc,
                list: vec![],
                _marker: PhantomData,
            },
        )
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
