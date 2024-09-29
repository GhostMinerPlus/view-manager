use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use edge_lib::util::engine::EdgeEngine;

mod inner {
    use std::{collections::HashMap, sync::Arc};

    use edge_lib::util::{engine::EdgeEngine, Path};

    use crate::err;

    use super::{Node, VNode, ViewProps};

    pub fn resize_child(
        vnode_id: u64,
        unique_id: &mut u64,
        vnode_mp: &mut HashMap<u64, VNode>,
        n_sz: usize,
    ) {
        let diff = vnode_mp.get(&vnode_id).unwrap().inner_node.child_v.len() as i32 - n_sz as i32;
        if diff < 0 {
            let mut last_id = *unique_id;
            vnode_mp
                .get_mut(&vnode_id)
                .unwrap()
                .inner_node
                .child_v
                .resize_with(n_sz, || {
                    let new_id = last_id;
                    last_id += 1;
                    Node::new(new_id)
                });
            for _ in 0..-diff {
                let new_id = *unique_id;
                *unique_id += 1;
                vnode_mp.insert(
                    new_id,
                    VNode::new(ViewProps {
                        class: format!(""),
                        props: json::Null,
                        child_v: vec![],
                    }),
                );
            }
        } else if diff > 0 {
            let extra = vnode_mp
                .get_mut(&vnode_id)
                .unwrap()
                .inner_node
                .child_v
                .split_off(n_sz);
            // TODO: on_delete_element
        }
    }

    pub async fn apply_layout(
        vnode_id: u64,
        unique_id: &mut u64,
        vnode_mp: &mut HashMap<u64, VNode>,
        view_props: &ViewProps,
        view_class: &HashMap<String, Vec<String>>,
        edge_engine: EdgeEngine,
        on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    ) -> err::Result<()> {
        resize_child(vnode_id, unique_id, vnode_mp, view_props.child_v.len());

        for i in 0..view_props.child_v.len() {
            let child_props = &view_props.child_v[i];
            let child_view_id = vnode_mp.get(&vnode_id).unwrap().inner_node.child_v[i].data;
            if vnode_mp
                .get(&child_view_id)
                .ok_or(err::Error::Other(format!(
                    "no vnode with id: {child_view_id}!"
                )))?
                .view_props
                != *child_props
            {
                super::apply_props(
                    child_view_id,
                    unique_id,
                    vnode_mp,
                    child_props,
                    view_class,
                    edge_engine.clone(),
                    on_create_element.clone(),
                    on_delete_element.clone(),
                    on_update_element.clone(),
                )
                .await?;
            }
        }
        let inner_node = &vnode_mp.get(&vnode_id).unwrap().inner_node;
        if vnode_mp.get(&inner_node.data).unwrap().view_props != *view_props {
            super::apply_props(
                inner_node.data,
                unique_id,
                vnode_mp,
                view_props,
                view_class,
                edge_engine,
                on_create_element,
                on_delete_element,
                on_update_element,
            )
            .await?;
        }
        Ok(())
    }

    ///
    pub async fn layout(
        view: &VNode,
        view_class: &HashMap<String, Vec<String>>,
        edge_engine: EdgeEngine,
    ) -> Option<ViewProps> {
        if let Some(script) = view_class.get(&view.view_props.class) {
            let mut edge_engine = edge_engine.divide();
            edge_engine
                .load(&view.view_props.props, &Path::from_str("$->$:input"))
                .await
                .unwrap();
            Some(super::util::execute_as_node(script, &view.view_props.child_v, edge_engine).await)
        } else {
            None
        }
    }
}
mod util;

pub mod err;

pub struct Node<Data> {
    pub data: Data,
    pub child_v: Vec<Node<Data>>,
}

impl<Data> Node<Data> {
    pub fn new(data: Data) -> Self {
        Self {
            data,
            child_v: vec![],
        }
    }

    pub fn new_with_child_v(data: Data, child_v: Vec<Node<Data>>) -> Self {
        Self { data, child_v }
    }
}

#[derive(PartialEq, Clone)]
pub struct ViewProps {
    pub class: String,
    pub props: json::JsonValue,
    pub child_v: Vec<ViewProps>,
}

pub struct VNode {
    pub view_props: ViewProps,
    pub inner_node: Node<u64>,
}

impl VNode {
    pub fn new(view_props: ViewProps) -> Self {
        Self {
            view_props,
            inner_node: Node::new(0),
        }
    }
}

pub fn apply_props<'a1, 'a2, 'a3, 'a4, 'f>(
    vnode_id: u64,
    unique_id: &'a1 mut u64,
    vnode_mp: &'a2 mut HashMap<u64, VNode>,
    view_props: &'a3 ViewProps,
    view_class: &'a4 HashMap<String, Vec<String>>,
    edge_engine: EdgeEngine,
    on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
) -> Pin<Box<impl Future<Output = err::Result<()>> + 'f>>
where
    'a1: 'f,
    'a2: 'f,
    'a3: 'f,
    'a4: 'f,
{
    Box::pin(async move {
        let vnode = vnode_mp.get_mut(&vnode_id).unwrap();

        if vnode.view_props.class != view_props.class {
            // TODO: on_delete_element && on_create_element
        }
        vnode.view_props = view_props.clone();

        if let Some(inner_props) = inner::layout(vnode, view_class, edge_engine.clone()).await {
            if vnode.inner_node.data == 0 {
                let new_id = *unique_id;
                *unique_id += 1;
                vnode_mp.get_mut(&vnode_id).unwrap().inner_node = Node::new(new_id);
                vnode_mp.insert(
                    new_id,
                    VNode::new(ViewProps {
                        class: format!(""),
                        props: json::Null,
                        child_v: vec![],
                    }),
                );
            }
            inner::apply_layout(
                vnode_id,
                unique_id,
                vnode_mp,
                &inner_props,
                view_class,
                edge_engine,
                on_create_element,
                on_delete_element,
                on_update_element,
            )
            .await?;
        } else {
            // update meta element
            inner::resize_child(vnode_id, unique_id, vnode_mp, view_props.child_v.len());
            for i in 0..view_props.child_v.len() {
                let child_props = &view_props.child_v[i];
                let child_view_id = vnode_mp.get(&vnode_id).unwrap().inner_node.child_v[i].data;
                if vnode_mp
                    .get(&child_view_id)
                    .ok_or(err::Error::Other(format!(
                        "no vnode with id: {child_view_id}!"
                    )))?
                    .view_props
                    != *child_props
                {
                    apply_props(
                        child_view_id,
                        unique_id,
                        vnode_mp,
                        child_props,
                        view_class,
                        edge_engine.clone(),
                        on_create_element.clone(),
                        on_delete_element.clone(),
                        on_update_element.clone(),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    })
}

pub struct ViewManager {
    unique_id: u64,
    vnode_mp: HashMap<u64, VNode>,
    view_class: HashMap<String, Vec<String>>,
    edge_engine: EdgeEngine,
    on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
}

impl ViewManager {
    pub async fn new(
        view_class: HashMap<String, Vec<String>>,
        entry: ViewProps,
        edge_engine: EdgeEngine,
        on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    ) -> err::Result<Self> {
        let mut unique_id = 0;
        let mut vnode_mp = HashMap::new();
        vnode_mp.insert(unique_id, VNode::new(entry.clone()));
        unique_id += 1;

        apply_props(
            0,
            &mut unique_id,
            &mut vnode_mp,
            &entry,
            &view_class,
            edge_engine.clone(),
            on_create_element.clone(),
            on_delete_element.clone(),
            on_update_element.clone(),
        )
        .await?;

        Ok(Self {
            unique_id,
            view_class,
            edge_engine,
            vnode_mp,
            on_create_element,
            on_delete_element,
            on_update_element,
        })
    }

    pub fn get_root(&self) -> &VNode {
        self.vnode_mp.get(&0).unwrap()
    }

    pub fn get_vnode(&self, id: &u64) -> Option<&VNode> {
        self.vnode_mp.get(id)
    }

    pub fn accept_event(&mut self, id: &u64, event: json::JsonValue) {
        // TODO: accept_event
    }

    pub fn get_vnode_mp(&self) -> &HashMap<u64, VNode> {
        &self.vnode_mp
    }
}
