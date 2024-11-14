use std::{collections::HashMap, pin::Pin};

use moon_class::{
    util::{executor::ClassExecutor, rs_2_str},
    AsClassManager, ClassManager, Fu,
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
    fn new(dm: impl AsClassManager + 'static) -> Self {
        Self {
            inner: InnerViewManager {
                unique_id: 0,
                vnode_mp: HashMap::new(),
            },
            cm: Box::new(dm),
        }
    }

    pub async fn init(&mut self, entry: ViewProps) {
        let root_id = self.new_vnode(0);
        self.apply_props(root_id, &entry, 0, true).await.unwrap();
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
            $props: {
                $onresize: <#dump($data()) = @new_size(@window);>,
                $onkeydown: <
                    [
                        {
                            $case: <#inner({ $left: w, $right: $key($data())}) := $result();>,
                            $then: <0.1 = $y($step);>
                        },
                        {
                            $case: <#inner({ $left: s, $right: $key($data())}) := $result();>,
                            $then: <-0.1 = $y($step);>
                        },
                        {
                            $case: <1 := $result();>,
                            $then: <0.0 = $y($step);>
                        }
                    ] = $switch();


                    0.0 = $x($step);
                    0.0 = $z($step);

                    #dump($step) = @new_step(@camera);

                    $() := $result();
                >
            }
        },
        {$class: Map}
    ]
} = $result();> = view(Main);

<{
    $class: div,
    $child: [
        {$class: Vision:light3, $props: {$position: [0.0, 5.0, 0.0]} },
        {$class: Box, $props: {$body_type: dynamic, $position: [-1.0, 2.0, -3.0], $color: [0.2, 0.4, 1.0]} },
        {$class: Box, $props: {$position: [-1.0, 0.0, -3.0], $color: [0.6, 1.0, 0.5]} }
    ]
} = $result();> = view(Map);

<#if({
    $left: $position($state()),
    $right: $position($props())
}) = $position();
#if({
    $left: $color($props()),
    $right: [0.2, 0.4, 1.0]
}) = $color();

{
    $class: div,
    $child: [
        {
            $class: Vision:cube3,
            $props: {
                $position: $position(),
                $color: $color()
            }
        },
        {
            $class: Physics:cube3,
            $props: {
                $position: $position($props()),
                $body_type: $body_type($props()),
                $onstep: <
                    @moon_world_pos($vnode_id()) := $position($state());

                    $state() = $result();
                >
            }
        }
    ]
} = $result();> = view(Box);
        "#,
        )
        .await
        .unwrap();

        vm.init(entry).await;

        println!("{}", inner::ser_html("  ", 0, &vm));

        vm.event_entry(1, "$onclick", &json::JsonValue::Null)
            .await
            .unwrap();

        println!("{}", inner::ser_html("  ", 0, &vm));
    })
}
