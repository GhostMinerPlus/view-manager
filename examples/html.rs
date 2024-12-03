use std::{
    collections::{BTreeMap, HashMap},
    pin::Pin,
};

use moon_class::{
    util::{executor::ClassExecutor, rs_2_str},
    AsClassManager, ClassManager, Fu,
};
use view_manager::{
    bean::{VNode, ViewProps},
    AsElementProvider, AsViewManager,
};

mod inner {
    use view_manager::AsViewManager;

    use crate::ViewManager;

    pub fn ser_html(space: &str, id: u64, vm: &ViewManager) -> String {
        let vnode = vm.get_vnode(&id).unwrap();
        if vnode.inner_id != 0 {
            // virtual container
            ser_html(&format!("{space}{space}"), vnode.inner_id, vm)
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
    dirty_vnode_v: BTreeMap<u64, Option<ViewProps>>,
}

impl ViewManager {
    fn new(dm: impl AsClassManager + 'static) -> Self {
        Self {
            inner: InnerViewManager {
                unique_id: 0,
                vnode_mp: HashMap::new(),
            },
            cm: Box::new(dm),
            dirty_vnode_v: BTreeMap::new(),
        }
    }

    pub async fn init(&mut self, entry: ViewProps) {
        let root_id = self.new_vnode(0);
        self.apply_props(root_id, Some(entry)).await.unwrap();
    }
}

impl AsClassManager for ViewManager {
    fn remove<'a, 'a1, 'a2, 'f>(
        &'a mut self,
        class: &'a1 str,
        source: &'a2 str,
        target_v: Vec<String>,
    ) -> Pin<Box<dyn moon_class::Fu<Output = moon_class::err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.cm.remove(class, source, target_v)
    }

    fn get<'a, 'a1, 'a2, 'f>(
        &'a self,
        class: &'a1 str,
        source: &'a2 str,
    ) -> Pin<Box<dyn moon_class::Fu<Output = moon_class::err::Result<Vec<String>>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
    {
        self.cm.get(class, source)
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
}

impl AsElementProvider for ViewManager {
    type H = u64;

    fn update_element(&mut self, id: u64, _class: &str, _props: &json::JsonValue) {
        log::debug!("update_element: id = {id}")
    }

    fn delete_element(&mut self, id: u64) {
        log::debug!("delete_element: id = {id}")
    }

    fn create_element(&mut self, vnode_id: u64, _class: &str, _props: &json::JsonValue) -> u64 {
        log::debug!("create_element: id = {vnode_id}");

        vnode_id
    }
}

impl AsViewManager for ViewManager {
    fn get_class_view<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 str,
    ) -> Pin<Box<dyn Fu<Output = Option<String>> + 'f>>
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

    fn dirty_vnode_v_mut(&mut self) -> &mut BTreeMap<u64, Option<ViewProps>> {
        &mut self.dirty_vnode_v
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("warn,html=debug,view_manager=debug,moon_class=debug"),
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

        let mut vm = ViewManager::new(ClassManager::new());

        let mut ce = ClassExecutor::new(&mut vm);

        ce.execute_script(
            r#"
<{
    $class: div,
    $child: [
        {
            $class: Input:window,
        },
        {$class: Map}
    ]
} = $result();> = view(Main);

<{
    $class: div,
    $child: [
        {$class: Vision:light3},
        {$class: Box},
        {$class: Box}
    ]
} = $result();> = view(Map);

<
    {
        $class: div,
        $child: [
            {
                $class: Vision:cube3,
            },
            {
                $class: Physics:cube3,
            }
        ],
        $props: {
            $type: set
        }
    } = $result();
> = view(Box);
        "#,
        )
        .await
        .unwrap();

        vm.init(entry).await;

        loop {
            let dirty_vnode_v = std::mem::take(vm.dirty_vnode_v_mut());

            if dirty_vnode_v.is_empty() {
                break;
            }

            for (vnode_id, view_props_op) in dirty_vnode_v {
                vm.apply_props(vnode_id, view_props_op).await.unwrap();
            }
        }

        println!("{}", inner::ser_html("  ", 0, &vm));
    })
}
