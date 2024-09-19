use std::{collections::HashMap, future::Future, pin::Pin};

use edge_lib::engine::{EdgeEngine, ScriptTree1};

mod inner {
    use std::collections::HashMap;

    use edge_lib::engine::{EdgeEngine, ScriptTree1};

    use super::{Node, VNode, ViewProps};

    pub async fn apply_layout(
        vnode_id: u64,
        unique_id: &mut u64,
        vnode_mp: &mut HashMap<u64, VNode>,
        props_node: &Node<ViewProps>,
        view_class: &HashMap<String, ScriptTree1>,
        edge_engine: EdgeEngine,
    ) {
        // let inner_node = vnode_mp
        //     .get(&vnode_id)
        //     .unwrap()
        //     .inner_node_op
        //     .as_ref()
        //     .unwrap();
        // inner_node
        //     .child_v
        //     .resize_with(props_node.child_v.len(), || {
        //         Node::new(VNode::new(ViewProps {
        //             class: format!(""),
        //             props: json::Null,
        //         }))
        //     });

        for i in 0..props_node.child_v.len() {
            let child_props_node = &props_node.child_v[i].data;
            let child_view_id = vnode_mp
                .get(&vnode_id)
                .unwrap()
                .inner_node_op
                .as_ref()
                .unwrap()
                .child_v[i]
                .data;
            if vnode_mp.get(&child_view_id).unwrap().view_props != *child_props_node {
                super::apply_props(
                    child_view_id,
                    unique_id,
                    vnode_mp,
                    child_props_node,
                    view_class,
                    edge_engine.clone(),
                )
                .await;
            }
        }
        let inner_node = vnode_mp
            .get(&vnode_id)
            .unwrap()
            .inner_node_op
            .as_ref()
            .unwrap();
        if vnode_mp.get(&inner_node.data).unwrap().view_props != props_node.data {
            super::apply_props(
                inner_node.data,
                unique_id,
                vnode_mp,
                &props_node.data,
                view_class,
                edge_engine.clone(),
            )
            .await;
        }
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
    class: String,
    props: json::JsonValue,
}

pub struct VNode {
    view_props: ViewProps,
    inner_node_op: Option<Node<u64>>,
}

impl VNode {
    pub fn new(view_props: ViewProps) -> Self {
        Self {
            view_props,
            inner_node_op: None,
        }
    }
}

pub fn apply_props<'a1, 'a2, 'a3, 'a4, 'f>(
    vnode_id: u64,
    unique_id: &'a1 mut u64,
    vnode_mp: &'a2 mut HashMap<u64, VNode>,
    props: &'a3 ViewProps,
    view_class: &'a4 HashMap<String, ScriptTree1>,
    edge_engine: EdgeEngine,
) -> Pin<Box<impl Future<Output = ()> + 'f>>
where
    'a1: 'f,
    'a2: 'f,
    'a3: 'f,
    'a4: 'f,
{
    Box::pin(async move {
        vnode_mp.get_mut(&vnode_id).unwrap().view_props = props.clone();

        if let Some(props_node) = inner::layout(
            vnode_mp.get(&vnode_id).unwrap(),
            view_class,
            edge_engine.clone(),
        )
        .await
        {
            if vnode_mp.get(&vnode_id).unwrap().inner_node_op.is_none() {
                vnode_mp.get_mut(&vnode_id).unwrap().inner_node_op = Some(Node::new(*unique_id));
                vnode_mp.insert(
                    *unique_id,
                    VNode::new(ViewProps {
                        class: format!(""),
                        props: json::Null,
                    }),
                );
                *unique_id += 1;
            }
            inner::apply_layout(
                vnode_id,
                unique_id,
                vnode_mp,
                &props_node,
                view_class,
                edge_engine,
            )
            .await;
        } else {
            // TODO: meta element
        }
    })
}

pub struct ViewManager {
    unique_id: u64,
    vnode_mp: HashMap<u64, VNode>,
    view_class: HashMap<String, ScriptTree1>,
    edge_engine: EdgeEngine,
}

impl ViewManager {
    pub async fn new(
        view_class: HashMap<String, ScriptTree1>,
        entry: ViewProps,
        edge_engine: EdgeEngine,
    ) -> Self {
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
        )
        .await;

        Self {
            unique_id,
            view_class,
            edge_engine,
            vnode_mp,
        }
    }

    pub fn get_root(&self) -> &VNode {
        self.vnode_mp.get(&0).unwrap()
    }

    pub fn get_vnode(&self, id: &u64) -> Option<&VNode> {
        self.vnode_mp.get(id)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use edge_lib::{
        data::MemDataManager,
        engine::{EdgeEngine, ScriptTree1},
    };

    use super::ViewManager;

    #[test]
    fn test() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut view_class = HashMap::new();
            view_class.insert(
                "Main".to_string(),
                ScriptTree1 {
                    script: vec![format!("$->$:output = ? _")],
                    name: "child".to_string(),
                    next_v: vec![
                        ScriptTree1 {
                            script: vec![format!("$->$:output = Body _")],
                            name: "class".to_string(),
                            next_v: vec![],
                        },
                        ScriptTree1 {
                            script: vec![format!("$->$:output = ? _")],
                            name: "props".to_string(),
                            next_v: vec![ScriptTree1 {
                                script: vec![format!("$->$:output = test _")],
                                name: "name".to_string(),
                                next_v: vec![],
                            }],
                        },
                    ],
                },
            );
            let entry = super::ViewProps {
                class: "Main".to_string(),
                props: json::Null,
            };
            let edge_engine = EdgeEngine::new(Arc::new(MemDataManager::new(None)), "root").await;
            let vm = ViewManager::new(view_class, entry, edge_engine).await;
            let root_view = vm.get_root();
            let inner = vm
                .get_vnode(&root_view.inner_node_op.as_ref().unwrap().data)
                .unwrap();
            assert_eq!(inner.view_props.class, "Body");
            assert_eq!(inner.view_props.props["name"][0], "test");
        });
    }
}
