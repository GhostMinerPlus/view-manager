use std::{collections::HashMap, future::Future, pin::Pin};

use moon_class::{
    util::{rs_2_str, str_of_value},
    AsClassManager, ClassExecutor, ClassManager,
};
use view_manager::{AsElementProvider, AsViewManager, VNode, ViewProps};

mod inner {
    use view_manager::AsViewManager;

    use crate::ViewManager;

    pub fn ser_html(space: &str, id: u64, vm: &ViewManager) -> String {
        let vnode = vm.get_vnode(&id).unwrap();
        if vnode.inner_node.data != 0 {
            // virtual container
            ser_html(&format!("{space}{space}"), vnode.inner_node.data, vm)
        } else {
            // meta container
            let mut html = format!("{space}<{}>", vnode.view_props.class);
            for child_node in &vnode.embeded_child_v {
                let child_html = ser_html(&format!("{space}{space}"), *child_node, vm);
                html = format!("{html}\n{child_html}");
            }
            format!("{html}\n{space}</{}>", vnode.view_props.class)
        }
    }
}

struct InnerViewManager {
    unique_id: u64,
    vnode_mp: HashMap<u64, VNode>,
}

struct ViewManager {
    inner: InnerViewManager,
    cm: Box<dyn AsClassManager>,
}

impl ViewManager {
    async fn new(entry: ViewProps, dm: Box<dyn AsClassManager>) -> Self {
        let mut this = Self {
            inner: InnerViewManager {
                unique_id: 0,
                vnode_mp: HashMap::new(),
            },
            cm: dm,
        };

        let root_id = this.new_vnode(0);
        this.apply_props(root_id, &entry, 0, true).await.unwrap();

        this
    }
}

impl AsClassManager for ViewManager {
    fn clear<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = moon_class::err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.cm.clear(class, pair)
    }

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        pair: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = moon_class::err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.cm.get(class, pair)
    }

    fn append<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        pair: &'a2 str,
        item_v: Vec<String>,
    ) -> Pin<Box<dyn moon_class::Fu<Output = moon_class::err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.cm.append(class, pair, item_v)
    }
    
    fn get_source<'a, 'a1, 'a2, 'f>(
        &'a self,
        target: &'a1 str,
        class: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = moon_class::err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f {
        self.cm.get_source(target, class)
    }
}

impl AsElementProvider for ViewManager {
    type H = u64;

    fn update_element(&mut self, id: u64, props: &ViewProps) {
        log::debug!("update_element: id = {id}")
    }

    fn delete_element(&mut self, id: u64) {
        log::debug!("delete_element: id = {id}")
    }

    fn create_element(&mut self, vnode_id: u64, class: &str) -> u64 {
        log::debug!("create_element: id = {vnode_id}");

        vnode_id
    }
}

impl AsViewManager for ViewManager {
    fn get_class_view<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 str,
    ) -> Pin<Box<dyn Future<Output = Option<String>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            let rs = self.get("view", class).await.unwrap();
            if rs.is_empty() {
                None
            } else {
                Some(rs_2_str(&rs))
            }
        })
    }

    fn get_vnode(&self, id: &u64) -> Option<&VNode> {
        self.inner.vnode_mp.get(id)
    }

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut VNode> {
        self.inner.vnode_mp.get_mut(id)
    }

    fn new_vnode(&mut self, context: u64) -> u64 {
        let new_id = self.inner.unique_id;
        self.inner.unique_id += 1;
        self.inner.vnode_mp.insert(new_id, VNode::new(context));
        new_id
    }

    fn rm_vnode(&mut self, id: u64) -> Option<VNode> {
        self.inner.vnode_mp.remove(&id)
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,html=debug,view_manager=debug"),
    )
    .init();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let entry = ViewProps {
            class: "Main".to_string(),
            props: json::Null,
        };
        let mut cm = Box::new(ClassManager::new());

        ClassExecutor::new(&mut *cm)
            .execute_script(&format!(
                "{} = view[Main];",
                str_of_value(
                    "test = $class[root];
                    1 = $props[root];

                    $class = $class[];
                    $props += $class[];
                    $child += $class[];
                    root = $source[];
                    #dump[] = $result[];"
                )
            ))
            .await
            .unwrap();

        let mut vm = ViewManager::new(entry, cm).await;

        println!("{}", inner::ser_html("  ", 0, &vm));

        vm.event_entry(1, "$onclick", &json::JsonValue::Null)
            .await
            .unwrap();

        println!("{}", inner::ser_html("  ", 0, &vm));
    })
}
