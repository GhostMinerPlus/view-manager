use std::{collections::HashMap, future::Future, pin::Pin};

use edge_lib::util::{
    data::{AsDataManager, AsStack, MemDataManager, TempDataManager},
    Path,
};
use view_manager::util::{AsViewManager, VNode, ViewProps};

mod inner {
    use view_manager::util::AsViewManager;

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
    dm: TempDataManager,
}

impl ViewManager {
    async fn new(entry: ViewProps, dm: Box<dyn AsDataManager>) -> Self {
        let mut this = Self {
            inner: InnerViewManager {
                unique_id: 0,
                vnode_mp: HashMap::new(),
            },
            dm: TempDataManager::new(dm),
        };

        let root_id = this.new_vnode(0);
        this.apply_props(root_id, &entry, 0, true).await.unwrap();

        this
    }
}

impl AsDataManager for ViewManager {
    fn get_auth(&self) -> &edge_lib::util::data::Auth {
        self.dm.get_auth()
    }

    fn append<'a, 'a1, 'f>(
        &'a mut self,
        path: &'a1 edge_lib::util::Path,
        item_v: Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        self.dm.append(path, item_v)
    }

    fn set<'a, 'a1, 'f>(
        &'a mut self,
        path: &'a1 edge_lib::util::Path,
        item_v: Vec<String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        self.dm.set(path, item_v)
    }

    fn get<'a, 'a1, 'f>(
        &'a self,
        path: &'a1 edge_lib::util::Path,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<Vec<String>>> + Send + 'f>,
    >
    where
        'a: 'f,
        'a1: 'f,
    {
        self.dm.get(path)
    }

    fn get_code_v<'a, 'a1, 'a2, 'f>(
        &'a self,
        root: &'a1 str,
        space: &'a2 str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<Vec<String>>> + Send + 'f>,
    >
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.dm.get_code_v(root, space)
    }

    fn call<'a, 'a1, 'a2, 'a3, 'a4, 'f>(
        &'a mut self,
        output: &'a1 edge_lib::util::Path,
        func: &'a2 str,
        input: &'a3 edge_lib::util::Path,
        input1: &'a4 edge_lib::util::Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
        'a3: 'f,
        'a4: 'f,
    {
        self.dm.call(output, func, input, input1)
    }
}

impl AsStack for ViewManager {
    fn push<'a, 'f>(
        &'a mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>> {
        self.dm.push()
    }

    fn pop<'a, 'f>(
        &'a mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::io::Result<()>> + Send + 'f>> {
        self.dm.pop()
    }
}

impl AsViewManager for ViewManager {
    fn get_class<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 str,
    ) -> Pin<Box<dyn Future<Output = Option<Vec<String>>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            let rs = self
                .get(&Path::from_str(&format!("$->{class}")))
                .await
                .unwrap();
            if rs.is_empty() {
                None
            } else {
                Some(rs)
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

    fn on_update_vnode_props(&mut self, id: u64, props: &ViewProps) {
        log::info!("on_update_vnode_props: {id}, {:?}", props);
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

        let entry = ViewProps {
            class: "Main".to_string(),
            props: json::Null,
        };
        let mut dm = Box::new(MemDataManager::new(None));

        dm.set(
            &Path::from_str("$->Main"),
            vec![
                format!("$->$:div = ? _"),
                //
                format!("$->$:div->$:class if $->$:state->$:name div"),
                format!("$->$:div->$:child = $child _"),
                //
                format!("$->$:root = ? _"),
                //
                format!("$->$:onclick = '$->$:state->$:name\\s=\\stest\\s_' _"),
                //
                format!("$->$:root->$:class = div _"),
                format!("$->$:root->$:props = ? _"),
                format!("$->$:root->$:child = $->$:div _"),
                //
                format!("$->$:root->$:props->$:onclick = $->$:onclick _"),
                //
                format!("$->$:output dump $->$:root $"),
            ],
        )
        .await
        .unwrap();

        let mut vm = ViewManager::new(entry, dm).await;

        println!("{}", inner::ser_html("  ", 0, &vm));

        vm.event_entry(1, "$:onclick", json::JsonValue::Null)
            .await
            .unwrap();

        println!("{}", inner::ser_html("  ", 0, &vm));
    })
}
