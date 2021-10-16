mod map;
mod vec;

use std::marker::PhantomData;

use bevy::{
    ecs::{prelude::*, system::SystemState},
    prelude::{BuildWorldChildren, Children, DespawnRecursiveExt},
};
use crossbeam_channel::Sender;
pub use map::TrackedMap;
pub use vec::TrackedVec;

use crate::{
    childable::CnufMarker,
    dom::ControlBundle,
    observer::{Observer, UninitObserver},
    prelude::Ctx,
    runtime::{UiScratchSpace, UpdateFunc},
};

use super::Childable;

pub trait TrackedObserverExt: Sized {
    fn each<F>(self, f: F) -> TrackedForeach<Self, F>;
}

#[rustfmt::skip]
impl<O, UO, T, Tt> TrackedObserverExt for UO
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = &'w Tt>,
    T: Send + Sync + 'static,
    Tt: Tracked<Item = T>,
{
    fn each<F>(self, f: F) -> TrackedForeach<Self, F> {
        TrackedForeach(self, f)
    }
}

#[derive(Clone)]
pub enum Diff<T> {
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

pub struct TrackedForeach<UO, F>(UO, F);

pub struct TrackedItemObserver<T: Send + Sync + 'static> {
    _marker: PhantomData<T>,
    entity: Entity,
}

impl<T: Send + Sync + 'static> UninitObserver for TrackedItemObserver<T> {
    type Observer = Self;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = uf(self, world);
        world
            .get_mut::<Element<T>>(self.entity)
            .unwrap()
            .ufs
            .push(uf.clone());
        uf
    }
}

impl<T: Send + Sync + 'static> Observer for TrackedItemObserver<T> {
    type Return<'w, 's> = (&'w T, usize);

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let element = world.get::<Element<T>>(self.entity).unwrap();
        ((&element.element, element.index), true)
    }
}

impl<T: Send + Sync + 'static> Clone for TrackedItemObserver<T> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
            entity: self.entity,
        }
    }
}
impl<T: Send + Sync + 'static> Copy for TrackedItemObserver<T> {}

pub trait Tracked: 'static {
    type Item;
    fn register(&self, sender: Sender<Diff<Self::Item>>);
}

pub struct TrackedMarker;

#[derive(Component)]
struct Element<T: Send + Sync + 'static> {
    index: usize,
    element: T,
    ufs: Vec<UpdateFunc>,
}

type Paramset<T> = SystemState<(
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, &'static mut Element<T>>,
    ResMut<'static, UiScratchSpace>,
)>;

#[rustfmt::skip]
impl<O, UO, T, F, C, Tt, M> Childable<(TrackedMarker, Tt, C, M)> for TrackedForeach<UO, F>
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = &'w Tt>,
    F: Fn(TrackedItemObserver<T>) -> C + Send + Sync + 'static,
    C: Childable<M>,
    T: Send + Sync + 'static,
    Tt: Tracked<Item = T>,
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let f = self.1;

        let c_parent = ctx
            .world
            .spawn()
            .insert_bundle(ControlBundle::default())
            .id();
        ctx.world.entity_mut(parent).push_children(&[c_parent]);

        self.0.register_self(ctx.world, |mut obs, world| {
            let (tv, _) = obs.get(world);
            let (tx, rx) = crossbeam_channel::unbounded();

            tv.register(tx);
            let mut paramset = Paramset::<T>::new(world);
            let (uf, marker) = UpdateFunc::new::<CnufMarker, _>(move |world| {
                let insert =
                    |world: &mut World, paramset: &mut Paramset<T>, e, i: Option<usize>| {
                        let index = i.unwrap_or_else(|| {
                            world
                                .get::<Children>(c_parent)
                                .map(|x| x.len())
                                .unwrap_or(0)
                        });
                        let element_entity = world
                            .spawn()
                            .insert(Element {
                                index,
                                element: e,
                                ufs: vec![],
                            })
                            .insert_bundle(ControlBundle::default())
                            .id();

                        world
                            .entity_mut(c_parent)
                            .insert_children(index, &[element_entity]);
                        let (children, mut element, mut ufs) = paramset.get_mut(world);
                        let entities = children.get(c_parent).unwrap();
                        for &entity in &entities[index + 1..] {
                            let mut element = element.get_mut(entity).unwrap();
                            ufs.register_update_funcs(element.ufs.iter().cloned());
                            element.index += 1;
                        }
                        let observer = TrackedItemObserver::<T> {
                            _marker: PhantomData,
                            entity: element_entity,
                        };
                        f(observer).insert(&mut Ctx {
                            current_entity: element_entity,
                            world,
                        })
                    };
                let remove = |world: &mut World, paramset: &mut Paramset<T>, i: Option<usize>| {
                    let children = world.get::<Children>(c_parent).unwrap();
                    let index = i.unwrap_or_else(|| children.len() - 1);
                    let to_despawn = children[index];
                    world.entity_mut(to_despawn).despawn_recursive();
                    let (children, mut element, mut ufs) = paramset.get_mut(world);
                    let entities = children.get(c_parent).unwrap();
                    for &entity in &entities[index..] {
                        let mut element = element.get_mut(entity).unwrap();
                        ufs.register_update_funcs(element.ufs.iter().cloned());
                        element.index -= 1;
                    }
                };
                for msg in rx.try_iter() {
                    match msg {
                        Diff::Init(list) => {
                            for element in list {
                                insert(world, &mut paramset, element, None);
                            }
                        }
                        Diff::Push(e) => insert(world, &mut paramset, e, None),
                        Diff::Pop => remove(world, &mut paramset, None),
                        Diff::Replace(e, i) => {
                            let (children, mut element, mut ufs) = paramset.get_mut(world);
                            let entities = children.get(c_parent).unwrap();
                            let mut element = element.get_mut(entities[i]).unwrap();
                            element.element = e;
                            ufs.register_update_funcs(element.ufs.iter().cloned());
                        }
                        // Diff::Switch(_, _) => todo!(),
                        Diff::Remove(i) => remove(world, &mut paramset, Some(i)),
                        Diff::Insert(e, i) => insert(world, &mut paramset, e, Some(i)),
                        Diff::Clear => {
                            for _ in 0..world
                                .get::<Children>(c_parent)
                                .map(|x| x.len())
                                .unwrap_or(0)
                            {
                                remove(world, &mut paramset, None);
                            }
                        }
                    }
                }
            });
            world.entity_mut(c_parent).insert(marker);
            uf.run(world);
            uf
        });
    }
}
