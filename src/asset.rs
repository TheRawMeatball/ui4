use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    ptr::NonNull,
};

use anyhow::bail;
use bevy::{
    asset::{Asset, AssetLoader},
    prelude::{warn, Component},
    reflect::TypeUuid,
    utils::HashMap,
};
use kdl::{KdlNode, KdlValue};

use crate::{insertable::Insertable, prelude::*};

#[derive(Debug, TypeUuid)]
#[uuid = "5c7d5f8a-f7b0-4e45-a09e-406c0372fea2"]
struct KdlAsset {}

macro_rules! bail_assert {
    ($e:expr, $msg:literal) => {
        if !$e {
            bail!($msg);
        }
    };
}

macro_rules! bail_assert_eq {
    ($a:expr, $b:expr, $msg:literal) => {
        if !($a == $b) {
            bail!($msg);
        }
    };
}

struct KdlAssetLoader {
    deser: HashMap<
        &'static str,
        // the dyn any should be Option<T>
        Box<dyn Fn(KdlNode, &mut dyn FnMut(&mut dyn Any)) + Send + Sync>,
    >,
}

impl KdlAssetLoader {
    fn register<T: Component>(
        &mut self,
        name: &'static str,
        // the dyn any should be Option<T>
        f: impl Fn(KdlNode, &mut dyn FnMut(&mut dyn Any)) + Send + Sync + 'static,
    ) {
        self.deser.insert(name, Box::new(f));
    }
}

impl AssetLoader for KdlAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let mut parsed = kdl::parse_document(std::str::from_utf8(bytes)?)?;
            bail_assert!(parsed.len() > 0, "Missing or too many nodes. You should have exactly two top-level nodes, `imports` and `widget`, in that order.");
            let imports = parsed.remove(0);
            bail_assert_eq!(
                &imports.name,
                "imports",
                "the first node should have the name `imports`"
            );
            bail_assert_eq!(
                imports.properties.len(),
                0,
                "There shouldn't be any properties on the `imports` node"
            );
            bail_assert_eq!(
                imports.values.len(),
                0,
                "There shouldn't be any values on the `imports` node"
            );
            for import in &imports.children {
                bail_assert_eq!(
                    import.properties.len(),
                    0,
                    "There shouldn't be any properties on an import node"
                );
                bail_assert_eq!(
                    import.values.len(),
                    0,
                    "There shouldn't be any values on an import node"
                );
                bail_assert_eq!(
                    import.children.len(),
                    0,
                    "There shouldn't be any children on an import node"
                );
            }

            let mut ops = Vec::new();

            for node in parsed {
                parse_node(node, &mut ops);
            }

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ui", "ui.kdl"]
    }
}

fn parse_node(node: KdlNode, ops: &mut OpVec) {
    for (name, prop) in node.properties {
        parse_component(
            KdlNode {
                name,
                values: vec![prop],
                properties: Default::default(),
                children: vec![],
            },
            ops,
        )
    }

    for child in node.children {
        match &child.name[..] {
            "-" => {
                let mut inner_ops = vec![];
                parse_node(child, &mut inner_ops);
                ops.push(Box::new(move |ctx, map| {
                    ctx.child(|mut inner| {
                        for op in &inner_ops {
                            inner = (op)(inner, map);
                        }
                        inner
                    })
                }));
            }
            "-i" => {
                ops.push(Box::new(move |ctx, map| {
                    if let Some(f) = map.get(&child.name) {
                        (f)(ctx)
                    } else {
                        warn!("Imported {} not given to asset", &child.name);
                        ctx
                    }
                }));
            }
            _ => {
                parse_component(child, ops);
            }
        }
    }
}

fn parse_component(node: KdlNode, ops: &mut OpVec) {
    // Deserialize node according to name, then push an op inserting it
}

type ImportMap = HashMap<String, Box<dyn Fn(Ctx) -> Ctx + Send + Sync>>;
type OpVec = Vec<Box<dyn for<'x> Fn(Ctx<'x>, &ImportMap) -> Ctx<'x> + Send + Sync>>;

struct DynWidget {
    build: Box<dyn for<'a> Fn(Ctx<'a>, &ImportMap) -> Ctx<'a> + Send + Sync>,
}

struct KdlWidget {
    kdl: kdl::KdlNode,
    imports: ImportMap,
}

fn kdl_widget() -> impl FnOnce(Ctx) -> Ctx {
    move |ctx| ctx
}
