use std::collections::HashMap;

use edge_lib::engine::{EdgeEngine, ScriptTree1};

mod inner {
    use std::collections::HashMap;

    use edge_lib::engine::{EdgeEngine, ScriptTree1};

    use super::{Node, View, ViewProps};

    #[async_recursion::async_recursion]
    pub async fn apply_layout(
        view_id: u64,
        unique_id: &mut u64,
        props_node: &Node<ViewProps>,
        view_class: &HashMap<String, ScriptTree1>,
        view_mp: &mut HashMap<u64, View>,
        edge_engine: EdgeEngine,
    ) {
        for i in 0..props_node.child_v.len() {
            let sub_view_id = view_mp.get(&view_id).unwrap().inner_view.child_v[i].data;
            let sub_props_node = &props_node.child_v[i];
            apply_layout(
                sub_view_id,
                unique_id,
                sub_props_node,
                view_class,
                view_mp,
                edge_engine.clone(),
            )
            .await;
        }

        view_mp
            .get_mut(&view_id)
            .unwrap()
            .inner_view
            .child_v
            .truncate(props_node.child_v.len());
        let sub_view_id = view_mp.get(&view_id).unwrap().inner_view.data;
        if view_mp.get(&sub_view_id).unwrap().view_props != props_node.data {
            super::apply_props(
                sub_view_id,
                unique_id,
                &props_node.data,
                view_class,
                view_mp,
                edge_engine,
            )
            .await;
        }
    }

    ///
    pub async fn layout(
        view_id: u64,
        view_class: &HashMap<String, ScriptTree1>,
        view_mp: &mut HashMap<u64, View>,
        edge_engine: EdgeEngine,
    ) -> Option<Node<ViewProps>> {
        let view = view_mp.get_mut(&view_id).unwrap();
        if let Some(script) = view_class.get(&view.view_props.class) {
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

pub struct View {
    view_props: ViewProps,
    state: json::JsonValue,
    parent_id: u64,
    inner_view: Node<u64>,
}

impl View {
    pub fn new(view_props: ViewProps) -> Self {
        Self {
            view_props,
            state: json::Null,
            parent_id: 0,
            inner_view: Node::new(0),
        }
    }
}

pub async fn apply_props(
    view_id: u64,
    unique_id: &mut u64,
    props: &ViewProps,
    view_class: &HashMap<String, ScriptTree1>,
    view_mp: &mut HashMap<u64, View>,
    edge_engine: EdgeEngine,
) {
    let view = view_mp.get_mut(&view_id).unwrap();
    view.view_props = props.clone();

    if let Some(props_node) = inner::layout(view_id, view_class, view_mp, edge_engine.clone()).await
    {
        inner::apply_layout(
            view_id,
            unique_id,
            &props_node,
            view_class,
            view_mp,
            edge_engine,
        )
        .await;
    }
}

pub struct ViewManager {
    unique_id: u64,
    view_class: HashMap<String, ScriptTree1>,
    view_mp: HashMap<u64, View>,
    edge_engine: EdgeEngine,
}

impl ViewManager {
    pub async fn new(
        view_class: HashMap<String, ScriptTree1>,
        entry: ViewProps,
        edge_engine: EdgeEngine,
    ) -> Self {
        let mut unique_id = 0;
        let mut view_mp = HashMap::new();

        view_mp.insert(unique_id, View::new(entry.clone()));
        unique_id += 1;

        apply_props(
            0,
            &mut unique_id,
            &entry,
            &view_class,
            &mut view_mp,
            edge_engine.clone(),
        )
        .await;

        Self {
            unique_id,
            view_class,
            view_mp,
            edge_engine,
        }
    }

    pub fn get_root(&self) -> &View {
        &self.view_mp[&0]
    }

    pub fn get_view(&self, id: &u64) -> Option<&View> {
        self.view_mp.get(id)
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
                    name: "inner".to_string(),
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
            let inner = vm.get_view(&root_view.inner_view.data).unwrap();
            assert_eq!(inner.view_props.class, "Body");
            assert_eq!(inner.view_props.props["name"][0], "test");
        });
    }
}
