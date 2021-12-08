use std::{marker::PhantomData, ops::Range, sync::atomic::AtomicU32};

use bevy::{
    ecs::{
        prelude::*,
        system::{
            lifetimeless::{Read, SQuery, SRes, Write},
            SystemState,
        },
    },
    prelude::{BuildWorldChildren, Children, DespawnRecursiveExt},
};
use crossbeam_channel::Receiver;

use crate::{
    dom::ControlBundle,
    lens::Identity,
    observer::{Observer, UninitObserver},
    prelude::{Ctx, WorldLens},
    runtime::{UiScratchSpace, UpdateFunc},
};

mod vec;
use super::Childable;
pub use vec::TrackedVec;

pub struct TrackedForeach<UO, F>(UO, F);

pub trait TrackedObserverExt: Sized {
    fn each<F>(self, f: F) -> TrackedForeach<Self, F>;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TrackedId(u32);

static TRACKED_ID_COUNTER: AtomicU32 = AtomicU32::new(0);
impl TrackedId {
    pub(crate) fn new() -> Self {
        Self(TRACKED_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

pub trait Tracked: 'static {
    type Item;
    fn register(&mut self) -> Receiver<Diff>;
    fn id(&self) -> TrackedId;

    fn get(&self, index: usize) -> &Self::Item;
    fn get_mut(&mut self, index: usize) -> &mut Self::Item;
    fn len(&self) -> usize;
}

impl<L> TrackedObserverExt for L
where
    L: WorldLens,
    L::Out: Tracked,
{
    fn each<F>(self, f: F) -> TrackedForeach<Self, F> {
        TrackedForeach(self, f)
    }
}

#[derive(Clone, Copy)]
pub enum Diff {
    // To be supported when Children supports it
    // Switch(usize, usize),
    Modify(usize),
    Remove(usize),
    Insert(usize),
    Clear,
}
pub struct TrackedMarker;

type Paramset = SystemState<(
    SQuery<Read<Children>>,
    SQuery<Write<Element>>,
    SRes<UiScratchSpace>,
)>;

pub struct TrackedItemLens<T, Parent> {
    parent: Parent,
    entity: Entity,
    _marker: PhantomData<T>,
}

impl<T, Parent: Copy> Clone for TrackedItemLens<T, Parent> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, Parent: Copy> Copy for TrackedItemLens<T, Parent> {}

impl<T, Parent> WorldLens for TrackedItemLens<T, Parent>
where
    Parent: WorldLens,
    Parent::Out: Tracked<Item = T>,
    T: Send + Sync + 'static,
{
    type UninitObserver = TrackedItemObserver<Parent>;
    type Observer = TrackedItemObserver<Parent>;
    type Lens = Identity<T>;
    type Out = T;

    fn get<'a>(&mut self, world: &'a World) -> &'a Self::Out {
        let index = world.get::<Element>(self.entity).unwrap().index;
        self.parent.get(world).get(index)
    }

    fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Self::Out {
        let index = world.get::<Element>(self.entity).unwrap().index;
        self.parent.get_mut(world).get_mut(index)
    }

    fn to_observer(self) -> (Self::UninitObserver, Self::Lens) {
        (
            TrackedItemObserver {
                parent: self.parent,
                entity: self.entity,
            },
            Identity(PhantomData),
        )
    }
}

#[derive(Copy, Clone)]
pub struct TrackedItemObserver<Parent> {
    parent: Parent,
    entity: Entity,
}

impl<Parent: WorldLens, T: 'static> UninitObserver for TrackedItemObserver<Parent>
where
    Parent::Out: Tracked<Item = T>,
{
    type Observer = TrackedItemObserver<Parent>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = uf(self, world);
        world
            .get_mut::<Element>(self.entity)
            .unwrap()
            .item_ufs
            .push(uf.clone());
        uf
    }
}

impl<'a, Parent: WorldLens, T: 'static> Observer<'a> for TrackedItemObserver<Parent>
where
    Parent::Out: Tracked<Item = T>,
{
    type Return = &'a T;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        let index = world.get::<Element>(self.entity).unwrap().index;
        (self.parent.get(world).get(index), true)
    }
}

#[derive(Clone, Copy)]
pub struct IndexObserver {
    entity: Entity,
}

impl UninitObserver for IndexObserver {
    type Observer = Self;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = uf(self, world);
        world
            .get_mut::<Element>(self.entity)
            .unwrap()
            .index_ufs
            .push(uf.clone());
        uf
    }
}

impl<'a> Observer<'a> for IndexObserver {
    type Return = usize;

    fn get(&'a mut self, world: &'a World) -> (Self::Return, bool) {
        (world.get::<Element>(self.entity).unwrap().index, true)
    }
}

#[derive(Component)]
struct Element {
    index: usize,
    item_ufs: Vec<UpdateFunc>,
    index_ufs: Vec<UpdateFunc>,
}

impl<F, C, M, L, T> Childable<(TrackedMarker, C, M)> for TrackedForeach<L, F>
where
    L: WorldLens,
    L::Out: Tracked<Item = T>,
    F: Fn(TrackedItemLens<T, L>, IndexObserver) -> C + Send + Sync + 'static,
    C: Childable<M>,
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let f = self.1;
        let world = &mut *ctx.world;

        let c_parent = world.spawn().insert_bundle(ControlBundle::default()).id();
        world.entity_mut(parent).push_children(&[c_parent]);

        let mut rx = None;
        let mut id = None;
        let world_lens = self.0;
        let (uo, _) = world_lens.to_observer();

        let mut paramset = Paramset::new(world);
        let mut diffs = vec![];

        let mut length = 0usize;
        uo.register_self(world, |_, world| {
            let (uf, marker) = UpdateFunc::new::<(), _>(move |world| {
                let tracked = world_lens.get_mut(world);

                if Some(tracked.id()) != id {
                    id = Some(tracked.id());
                    rx = Some(tracked.register());
                    // the tracked object has changed - despawn everything and start fresh

                    rx.as_mut().unwrap().try_iter().for_each(drop);
                    // TODO: uncomment once pr merged
                    // world.entity_mut(c_parent).despawn_children();

                    for i in 0..tracked.len() {
                        let manager = world
                            .spawn()
                            .insert(Element {
                                index: i,
                                item_ufs: vec![],
                                index_ufs: vec![],
                            })
                            .insert_bundle(ControlBundle::default())
                            .id();

                        world.entity_mut(c_parent).insert_children(i, &[manager]);

                        let lens = TrackedItemLens {
                            parent: world_lens,
                            entity: manager,
                            _marker: PhantomData,
                        };
                        let index_observer = IndexObserver { entity: manager };

                        let childable = f(lens, index_observer);
                        childable.insert(&mut Ctx {
                            world,
                            current_entity: manager,
                        });
                    }
                } else {
                    diffs.clear();
                    diffs.extend(rx.as_mut().unwrap().try_iter());

                    for &diff in &diffs {
                        match diff {
                            Diff::Insert(i) => {
                                length += 1;
                                let manager = world
                                    .spawn()
                                    .insert(Element {
                                        index: i,
                                        item_ufs: vec![],
                                        index_ufs: vec![],
                                    })
                                    .insert_bundle(ControlBundle::default())
                                    .id();

                                world.entity_mut(c_parent).insert_children(i, &[manager]);

                                change_indexes(
                                    i + 1..length as usize,
                                    1,
                                    &mut paramset,
                                    world,
                                    c_parent,
                                );

                                let lens = TrackedItemLens {
                                    parent: world_lens,
                                    entity: manager,
                                    _marker: PhantomData,
                                };
                                let index_observer = IndexObserver { entity: manager };

                                let childable = f(lens, index_observer);
                                childable.insert(&mut Ctx {
                                    world,
                                    current_entity: manager,
                                });
                            }
                            Diff::Remove(i) => {
                                length -= 1;
                                world
                                    .entity_mut(
                                        world.entity(c_parent).get::<Children>().unwrap()[i],
                                    )
                                    .despawn_recursive();

                                change_indexes(
                                    i..length as usize,
                                    -1,
                                    &mut paramset,
                                    world,
                                    c_parent,
                                );
                            }
                            Diff::Modify(i) => {
                                let (children_q, mut element_q, scratch_space) =
                                    paramset.get_mut(world);
                                let children =
                                    children_q.get(c_parent).map(|x| &**x).unwrap_or(&[]);
                                scratch_space.process_list(
                                    &mut element_q.get_mut(children[i]).unwrap().item_ufs,
                                );
                            }
                            Diff::Clear => {
                                length = 0;
                                // TODO: uncomment once pr merged
                                // world.entity_mut(c_parent).despawn_children();
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

fn change_indexes(
    range: Range<usize>,
    by: isize,
    paramset: &mut Paramset,
    world: &mut World,
    c_parent: Entity,
) {
    let (children_q, mut element_q, scratch_space) = paramset.get_mut(world);
    let children = children_q.get(c_parent).map(|x| &**x).unwrap_or(&[]);
    for &e in &children[range] {
        let mut e = element_q.get_mut(e).unwrap();
        e.index = (e.index as isize + by) as usize;
        scratch_space.process_list(&mut e.index_ufs);
    }
}
