use std::{collections::HashMap, sync::Arc};

use edge_lib::{
    data::MemDataManager,
    engine::{EdgeEngine, ScriptTree1},
};
use view_manager::{ViewManager, ViewProps};

mod inner {
    use std::collections::HashMap;

    use view_manager::VNode;

    pub fn ser_html(space: &str, id: u64, vnode_mp: &HashMap<u64, VNode>) -> String {
        let vnode = vnode_mp.get(&id).unwrap();
        if vnode.inner_node.data != 0 {
            ser_html(&format!("{space}{space}"), vnode.inner_node.data, vnode_mp)
        } else {
            let mut html = format!(
                "{space}<{}>",
                vnode.view_props.class
            );
            
            format!(
                "{html}\n{space}</{}>",
                vnode.view_props.class
            )
        }
    }
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("warn,html=debug,view-manager=debug"),
        )
        .init();

        let mut view_class = HashMap::new();
        view_class.insert(
            "Main".to_string(),
            ScriptTree1 {
                script: vec![format!("$->$:output = ? _")],
                name: "child".to_string(),
                next_v: vec![
                    ScriptTree1 {
                        script: vec![format!("$->$:output = div _")],
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
                    ScriptTree1 {
                        script: vec![format!("$->$:output = ? _")],
                        name: "child".to_string(),
                        next_v: vec![
                            ScriptTree1 {
                                script: vec![format!("$->$:output = div _")],
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
                ],
            },
        );
        let entry = ViewProps {
            class: "Main".to_string(),
            props: json::Null,
        };
        let edge_engine = EdgeEngine::new(Arc::new(MemDataManager::new(None)), "root").await;
        let vm = ViewManager::new(
            view_class,
            entry,
            edge_engine,
            Arc::new(|id, vnode_mp| {}),
            Arc::new(|id, vnode_mp| {}),
            Arc::new(|id, vnode_mp| {}),
        )
        .await;

        println!("{}", inner::ser_html("    ", 0, vm.get_vnode_mp()));
    })
}
