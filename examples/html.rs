use std::{collections::HashMap, sync::Arc};

use edge_lib::util::{
    data::{AsDataManager, MemDataManager, TempDataManager},
    engine::{AsEdgeEngine, EdgeEngine},
};
use view_manager::{AsViewManager, VNode, ViewProps};

mod inner {
    use view_manager::AsViewManager;

    use crate::ViewManager;

    pub fn ser_html(space: &str, id: u64, vnode_mp: &ViewManager) -> String {
        let vnode = vnode_mp.get_vnode(&id).unwrap();
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

struct InnerViewManager {
    unique_id: u64,
    vnode_mp: HashMap<u64, VNode>,
    view_class: HashMap<String, Vec<String>>,
}

struct ViewManager {
    inner: InnerViewManager,
    dm: TempDataManager,
}

impl ViewManager {
    async fn new(
        view_class: HashMap<String, Vec<String>>,
        entry: ViewProps,
        dm: Arc<dyn AsDataManager>,
    ) -> Self {
        let mut unique_id = 0;
        let mut vnode_mp = HashMap::new();
        vnode_mp.insert(unique_id, VNode::new(entry.clone()));
        unique_id += 1;

        let mut this = Self {
            inner: InnerViewManager {
                unique_id,
                view_class,
                vnode_mp,
            },
            dm: TempDataManager::new(dm),
        };

        this.apply_props(0, &entry).await.unwrap();

        this
    }
}

impl AsEdgeEngine for ViewManager {
    fn get_dm(&self) -> &TempDataManager {
        &self.dm
    }

    fn reset(&mut self) {
        self.dm.temp = Arc::new(MemDataManager::new(None));
    }

    fn get_dm_mut(&mut self) -> &mut TempDataManager {
        &mut self.dm
    }
}

impl AsViewManager for ViewManager {
    fn get_class(&self, class: &str) -> Option<&Vec<String>> {
        self.inner.view_class.get(class)
    }

    fn get_vnode(&self, id: &u64) -> Option<&VNode> {
        self.inner.vnode_mp.get(id)
    }

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut VNode> {
        self.inner.vnode_mp.get_mut(id)
    }

    fn new_vnode(&mut self) -> u64 {
        let new_id = self.inner.unique_id;
        self.inner.unique_id += 1;
        self.inner.vnode_mp.insert(
            new_id,
            VNode::new(ViewProps {
                class: format!(""),
                props: json::Null,
                child_v: vec![],
            }),
        );
        new_id
    }

    fn rm_vnode(&mut self, id: u64) -> Option<VNode> {
        self.inner.vnode_mp.remove(&id)
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
        )
        .await;

        vm.event_entry(1, "$:onclick", json::JsonValue::Null)
            .await
            .unwrap();

        println!("{}", inner::ser_html("  ", 0, &vm));
    })
}
