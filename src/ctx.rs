use std::marker::PhantomData;

use bevy::ecs::prelude::*;

use crate::{
    childable::Childable,
    dom::ControlBundle,
    insertable::Insertable,
    lens::ComponentLens,
    observer::{ComponentExistsObserver, Observer, UninitObserver},
    runtime::{UfMarker, UiScratchSpace, UpdateFunc},
};

/// Entry point for creating entities. Having a `Ctx` means you control all components on this entity, and all
/// its children.
pub struct Ctx<'a> {
    pub(crate) world: &'a mut World,
    pub(crate) current_entity: Entity,
}

pub struct McCtx<'a> {
    pub(crate) world: &'a mut World,
    pub(crate) get_new_child: &'a mut dyn FnMut(&mut World) -> Entity,
}

/// Stands for "Multi Child Context". Represents being able to add children to a pre-existing entity, but modifying said
/// entity isn't possible.
impl McCtx<'_> {
    // TODO: bikeshed name
    pub fn c(&mut self, f: impl FnOnce(Ctx) -> Ctx) -> &mut Self {
        let new_child = (self.get_new_child)(self.world);
        f(Ctx {
            current_entity: new_child,
            world: self.world,
        });
        self
    }

    pub fn dyn_group<M>(&mut self, children: impl Childable<M>) -> &mut Self {
        self.c(|ctx: Ctx| ctx.with_bundle(ControlBundle::default()).children(children))
    }
}

impl Ctx<'_> {
    /// Statically inserts a component, or sets up reactive-ness if given a reactive template.
    #[track_caller]
    pub fn with<T: Component, M>(mut self, item: impl Insertable<T, M>) -> Self {
        item.insert_ui_val(&mut self);
        self
    }

    pub fn with_modified<T, O, F>(mut self, initial: T, observer: O) -> Self
    where
        T: Component,
        O: UninitObserver,
        for<'a> O::Observer: Observer<'a, Return = F>,
        F: FnOnce(&mut T) + 'static,
    {
        initial.insert_ui_val(&mut self);
        let entity = self.current_entity;
        let uf = observer.register_self(self.world, |mut observer, world| {
            let mut first = true;
            let (uf, marker) = UpdateFunc::new::<T, _>(move |world| {
                let (f, changed) = observer.get(world);
                if !changed && !first {
                    return;
                }
                first = false;
                world.resource_scope(|world, mut ctx: Mut<UiScratchSpace>| {
                    let mut e = world.entity_mut(entity);
                    e.get_mut::<UfMarker<T>>().unwrap().trigger(&mut ctx);
                    f(e.get_mut::<T>().unwrap().into_inner());
                })
            });
            world.entity_mut(entity).insert(marker);
            uf
        });
        uf.run(&mut self.world);
        self
    }

    /// Inserts a bundle *statically*. If you need certain parts of this bundle to be reactive,
    /// use [`Ctx::with`] for those particular components instead.
    pub fn with_bundle(self, bundle: impl Bundle) -> Self {
        self.world
            .entity_mut(self.current_entity)
            .insert_bundle(bundle);
        self
    }

    pub fn child(self, f: impl FnOnce(Ctx) -> Ctx) -> Self {
        self.children(|ctx: &mut McCtx| {
            ctx.c(f);
        })
    }

    pub fn children<M>(mut self, children: impl Childable<M>) -> Self {
        children.insert(&mut self);
        self
    }

    /// Gets an lens for a component on the entity being built. Panics if the component is removed
    /// and the lens is still in-use.
    pub fn component<T: Component>(&self) -> ComponentLens<T> {
        ComponentLens(self.current_entity, PhantomData)
    }

    /// Gets an observer for whether the current entity has a particular component. Most useful with marker
    /// components.
    pub fn has_component<T: Send + Sync + 'static>(&self) -> ComponentExistsObserver<T> {
        ComponentExistsObserver(self.current_entity, PhantomData)
    }

    pub fn current_entity(&self) -> Entity {
        self.current_entity
    }
}
