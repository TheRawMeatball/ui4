use std::marker::PhantomData;

use bevy::{ecs::prelude::*, prelude::ControlBundle};

use crate::{
    childable::Childable,
    insertable::Insertable,
    observer::{ComponentExistsObserver, ComponentObserver},
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

    /// Gets an observer for a component on the entity being built. Panics if the component is removed
    /// and the observer is still in-use.
    pub fn component<T: Send + Sync + 'static>(&self) -> ComponentObserver<T> {
        ComponentObserver(self.current_entity, PhantomData)
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
