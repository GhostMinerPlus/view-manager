use std::{collections::HashMap, sync::Arc};

use edge_lib::util::{
    data::{AsDataManager, MemDataManager},
    engine::{AsEdgeEngine, EdgeEngine},
};
use view_manager::{ViewManager, ViewProps};

mod inner {
    use std::collections::HashMap;

    use view_manager::VNode;

    pub fn ser_html(space: &str, id: u64, vnode_mp: &HashMap<u64, VNode>) -> String {
        let vnode = vnode_mp.get(&id).unwrap();
        if vnode.inner_node.data != 0 {
            // virtual container
            ser_html(&format!("{space}{space}"), vnode.inner_node.data, vnode_mp)
        } else {
            // meta container
            let mut html = format!("{space}<{}>", vnode.view_props.class);
            for child_node in &vnode.inner_node.child_v {
                let child_html = ser_html(&format!("{space}{space}"), child_node.data, vnode_mp);
                html = format!("{html}\n{child_html}");
            }
            format!("{html}\n{space}</{}>", vnode.view_props.class)
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
            env_logger::Env::default().default_filter_or("warn,html=debug,view_manager=debug"),
        )
        .init();

        let edge_engine = EdgeEngine::new(Arc::new(MemDataManager::new(None)), "root").await;

        let mut view_class = HashMap::new();

        view_class.insert(
            "Main".to_string(),
            vec![
                format!("$->$:div = ? _"),
                //
                format!("$->$:div->$:class = div _"),
                format!("$->$:div->$:child = $child _"),
                //
                format!("$->$:root = ? _"),
                //
                format!("$->$:onclick = '$->$:output\\s+\\s1\\s1' _"),
                //
                format!("$->$:root->$:class = div _"),
                format!("$->$:root->$:props = ? _"),
                format!("$->$:root->$:child = $->$:div _"),
                //
                format!("$->$:root->$:props->$:onclick = $->$:onclick _"),
                //
                format!("$->$:output dump $->$:root $"),
            ],
        );

        let entry = ViewProps {
            class: "Main".to_string(),
            props: json::Null,
            child_v: vec![
                ViewProps {
                    class: "input".to_string(),
                    props: json::Null,
                    child_v: vec![],
                },
                ViewProps {
                    class: "input".to_string(),
                    props: json::Null,
                    child_v: vec![],
                },
                ViewProps {
                    class: "input".to_string(),
                    props: json::Null,
                    child_v: vec![],
                },
            ],
        };

        let mut vm = ViewManager::new(
            view_class,
            entry,
            edge_engine
                .get_dm()
                .divide(edge_engine.get_dm().get_auth().clone()),
            Arc::new(|id, vnode_mp| {}),
            Arc::new(|id, vnode_mp| {}),
            Arc::new(|id, vnode_mp| {}),
        )
        .await
        .unwrap();

        vm.event_entry(&1, "$:onclick", json::JsonValue::Null).await;

        println!("{}", inner::ser_html("  ", 0, vm.get_vnode_mp()));
    })
}
