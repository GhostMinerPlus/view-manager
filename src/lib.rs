use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use edge_lib::engine::{EdgeEngine, ScriptTree1};

mod inner {
    use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

    use edge_lib::engine::{EdgeEngine, ScriptTree1};

    use super::{Node, VNode, ViewProps};

    pub fn apply_layout<'a, 'f>(
        vnode_id: u64,
        unique_id: &'a mut u64,
        vnode_mp: &'a mut HashMap<u64, VNode>,
        props_node: &'a Node<ViewProps>,
        view_class: &'a HashMap<String, ScriptTree1>,
        edge_engine: EdgeEngine,
        on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'f>>
    where
        'a: 'f,
    {
        Box::pin(async move {
            let diff = vnode_mp.get(&vnode_id).unwrap().inner_node.child_v.len() as i32
                - props_node.child_v.len() as i32;
            if diff < 0 {
                let mut last_id = *unique_id;
                vnode_mp
                    .get_mut(&vnode_id)
                    .unwrap()
                    .inner_node
                    .child_v
                    .resize_with(props_node.child_v.len(), || {
                        let new_id = last_id;
                        last_id += 1;
                        Node::new(new_id)
                    });
                for _ in 0..diff {
                    let new_id = *unique_id;
                    *unique_id += 1;
                    vnode_mp.insert(
                        new_id,
                        VNode::new(ViewProps {
                            class: format!(""),
                            props: json::Null,
                        }),
                    );
                }
            } else if diff > 0 {
                let extra = vnode_mp
                    .get_mut(&vnode_id)
                    .unwrap()
                    .inner_node
                    .child_v
                    .split_off(props_node.child_v.len());
                // TODO: on_delete_element
            }

            for i in 0..props_node.child_v.len() {
                let child_props_node = &props_node.child_v[i];
                let child_view_id = vnode_mp.get(&vnode_id).unwrap().inner_node.child_v[i].data;
                if vnode_mp.get(&child_view_id).unwrap().view_props != child_props_node.data {
                    super::apply_props(
                        child_view_id,
                        unique_id,
                        vnode_mp,
                        child_props_node,
                        view_class,
                        edge_engine.clone(),
                        on_create_element.clone(),
                        on_delete_element.clone(),
                        on_update_element.clone(),
                    )
                    .await;
                }
            }
            let inner_node = &vnode_mp.get(&vnode_id).unwrap().inner_node;
            if vnode_mp.get(&inner_node.data).unwrap().view_props != props_node.data {
                super::apply_props(
                    inner_node.data,
                    unique_id,
                    vnode_mp,
                    props_node,
                    view_class,
                    edge_engine,
                    on_create_element,
                    on_delete_element,
                    on_update_element,
                )
                .await;
            }
        })
    }

    ///
    pub async fn layout(
        view: &VNode,
        view_class: &HashMap<String, ScriptTree1>,
        edge_engine: EdgeEngine,
    ) -> Option<Node<ViewProps>> {
        if let Some(script) = view_class.get(&view.view_props.class) {
            let edge_engine = edge_engine.divide();
            // TODO: input props

            super::util::execute_as_node(script, edge_engine).await
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
    props_node: &'a3 Node<ViewProps>,
    view_class: &'a4 HashMap<String, ScriptTree1>,
    edge_engine: EdgeEngine,
    on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
) -> Pin<Box<impl Future<Output = ()> + 'f>>
where
    'a1: 'f,
    'a2: 'f,
    'a3: 'f,
    'a4: 'f,
{
    Box::pin(async move {
        let inner_node = vnode_mp.get_mut(&vnode_id).unwrap();
        if inner_node.view_props.class != props_node.data.class {
            // TODO: on_delete_element && on_create_element
        }
        inner_node.view_props = props_node.data.clone();

        if let Some(props_node) = inner::layout(
            vnode_mp.get(&vnode_id).unwrap(),
            view_class,
            edge_engine.clone(),
        )
        .await
        {
            if vnode_mp.get(&vnode_id).unwrap().inner_node.data == 0 {
                let new_id = *unique_id;
                *unique_id += 1;
                vnode_mp.get_mut(&vnode_id).unwrap().inner_node = Node::new(new_id);
                vnode_mp.insert(
                    new_id,
                    VNode::new(ViewProps {
                        class: format!(""),
                        props: json::Null,
                    }),
                );
            }
            inner::apply_layout(
                vnode_id,
                unique_id,
                vnode_mp,
                &props_node,
                view_class,
                edge_engine,
                on_create_element,
                on_delete_element,
                on_update_element,
            )
            .await;
        } else {
            // TODO: update meta element
        }
    })
}

pub struct ViewManager {
    unique_id: u64,
    vnode_mp: HashMap<u64, VNode>,
    view_class: HashMap<String, ScriptTree1>,
    edge_engine: EdgeEngine,
    on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
}

impl ViewManager {
    pub async fn new(
        view_class: HashMap<String, ScriptTree1>,
        entry: ViewProps,
        edge_engine: EdgeEngine,
        on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
        on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>)>,
    ) -> Self {
        let mut unique_id = 0;
        let mut vnode_mp = HashMap::new();
        vnode_mp.insert(unique_id, VNode::new(entry.clone()));
        unique_id += 1;

        apply_props(
            0,
            &mut unique_id,
            &mut vnode_mp,
            &Node::new(entry),
            &view_class,
            edge_engine.clone(),
            on_create_element.clone(),
            on_delete_element.clone(),
            on_update_element.clone(),
        )
        .await;

        Self {
            unique_id,
            view_class,
            edge_engine,
            vnode_mp,
            on_create_element,
            on_delete_element,
            on_update_element,
        }
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
