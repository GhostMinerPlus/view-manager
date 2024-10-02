use std::{cell::RefCell, collections::HashMap, future::Future, pin::Pin, sync::Arc};

use edge_lib::util::{
    data::{AsDataManager, MemDataManager, TempDataManager},
    engine::AsEdgeEngine,
    Path,
};

mod inner {
    use std::collections::HashMap;

    use edge_lib::util::{engine::AsEdgeEngine, Path};

    use crate::{err, ViewManager};

    use super::{Node, VNode, ViewProps};

    pub fn resize_child(
        vnode_id: u64,
        unique_id: &mut u64,
        vnode_mp: &mut HashMap<u64, VNode>,
        n_sz: usize,
    ) {
        let diff = vnode_mp.get(&vnode_id).unwrap().inner_node.child_v.len() as i32 - n_sz as i32;
        if diff < 0 {
            let mut last_id = *unique_id;
            vnode_mp
                .get_mut(&vnode_id)
                .unwrap()
                .inner_node
                .child_v
                .resize_with(n_sz, || {
                    let new_id = last_id;
                    last_id += 1;
                    Node::new(new_id)
                });
            for _ in 0..-diff {
                let new_id = *unique_id;
                *unique_id += 1;
                vnode_mp.insert(
                    new_id,
                    VNode::new(ViewProps {
                        class: format!(""),
                        props: json::Null,
                        child_v: vec![],
                    }),
                );
            }
        } else if diff > 0 {
            let extra = vnode_mp
                .get_mut(&vnode_id)
                .unwrap()
                .inner_node
                .child_v
                .split_off(n_sz);
            // TODO: on_delete_element
        }
    }

    pub async fn apply_layout(
        vm: &mut ViewManager,
        vnode_id: u64,
        view_props: &ViewProps,
    ) -> err::Result<()> {
        resize_child(
            vnode_id,
            &mut vm.inner.unique_id,
            &mut vm.inner.vnode_mp,
            view_props.child_v.len(),
        );

        for i in 0..view_props.child_v.len() {
            let child_props = &view_props.child_v[i];
            let child_view_id =
                vm.inner.vnode_mp.get(&vnode_id).unwrap().inner_node.child_v[i].data;
            if vm
                .inner
                .vnode_mp
                .get(&child_view_id)
                .ok_or(err::Error::Other(format!(
                    "no vnode with id: {child_view_id}!"
                )))?
                .view_props
                != *child_props
            {
                super::apply_props(vm, child_view_id, child_props).await?;
            }
        }
        let inner_node = &vm.inner.vnode_mp.get(&vnode_id).unwrap().inner_node;
        if vm.inner.vnode_mp.get(&inner_node.data).unwrap().view_props != *view_props {
            super::apply_props(vm, inner_node.data, view_props).await?;
        }
        Ok(())
    }

    ///
    pub async fn layout(view: &VNode, vm: &mut ViewManager) -> Option<ViewProps> {
        if let Some(script) = vm.inner.view_class.get(&view.view_props.class) {
            vm.load(&view.view_props.props, &Path::from_str("$->$:input"))
                .await
                .unwrap();
            let node =
                super::util::execute_as_node(&script.clone(), &view.view_props.child_v, vm).await;

            Some(node)
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

impl<Data: Clone> Clone for Node<Data> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            child_v: self.child_v.clone(),
        }
    }
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
    pub child_v: Vec<ViewProps>,
}

#[derive(Clone)]
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

pub fn apply_props<'a1, 'a2, 'f>(
    vm: &'a1 mut ViewManager,
    vnode_id: u64,
    view_props: &'a2 ViewProps,
) -> Pin<Box<impl Future<Output = err::Result<()>> + 'f>>
where
    'a1: 'f,
    'a2: 'f,
{
    Box::pin(async move {
        let vnode = vm.inner.vnode_mp.get_mut(&vnode_id).unwrap();

        if vnode.view_props.class != view_props.class {
            // TODO: on_delete_element && on_create_element
        }
        vnode.view_props = view_props.clone();

        let vnode = vm.inner.vnode_mp.get(&vnode_id).unwrap().clone();

        if let Some(inner_props) = inner::layout(&vnode.clone(), vm).await {
            let vnode = vm.inner.vnode_mp.get(&vnode_id).unwrap();
            if vnode.inner_node.data == 0 {
                let new_id = vm.inner.unique_id;
                vm.inner.unique_id += 1;
                vm.inner.vnode_mp.get_mut(&vnode_id).unwrap().inner_node = Node::new(new_id);
                vm.inner.vnode_mp.insert(
                    new_id,
                    VNode::new(ViewProps {
                        class: format!(""),
                        props: json::Null,
                        child_v: vec![],
                    }),
                );
            }
            inner::apply_layout(vm, vnode_id, &inner_props).await?;
        } else {
            // update meta element
            inner::resize_child(
                vnode_id,
                &mut vm.inner.unique_id,
                &mut vm.inner.vnode_mp,
                view_props.child_v.len(),
            );
            for i in 0..view_props.child_v.len() {
                let child_props = &view_props.child_v[i];
                let child_view_id =
                    vm.inner.vnode_mp.get(&vnode_id).unwrap().inner_node.child_v[i].data;
                if vm
                    .inner
                    .vnode_mp
                    .get(&child_view_id)
                    .ok_or(err::Error::Other(format!(
                        "no vnode with id: {child_view_id}!"
                    )))?
                    .view_props
                    != *child_props
                {
                    apply_props(vm, child_view_id, child_props).await?;
                }
            }
        }
        Ok(())
    })
}

pub struct InnerViewManager {
    unique_id: u64,
    vnode_mp: HashMap<u64, VNode>,
    view_class: HashMap<String, Vec<String>>,
    on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>) + Send + Sync>,
    on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>) + Send + Sync>,
    on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>) + Send + Sync>,
}

pub struct ViewManager {
    inner: InnerViewManager,
    dm: TempDataManager,
}

impl ViewManager {
    pub async fn new(
        view_class: HashMap<String, Vec<String>>,
        entry: ViewProps,
        dm: Arc<dyn AsDataManager>,
        on_create_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>) + Send + Sync>,
        on_delete_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>) + Send + Sync>,
        on_update_element: Arc<dyn Fn(u64, &HashMap<u64, VNode>) + Send + Sync>,
    ) -> err::Result<Self> {
        let mut unique_id = 0;
        let mut vnode_mp = HashMap::new();
        vnode_mp.insert(unique_id, VNode::new(entry.clone()));
        unique_id += 1;

        let mut this = Self {
            inner: InnerViewManager {
                unique_id,
                view_class,
                vnode_mp,
                on_create_element,
                on_delete_element,
                on_update_element,
            },
            dm: TempDataManager::new(dm),
        };

        apply_props(&mut this, 0, &entry).await?;

        Ok(this)
    }

    pub fn get_root(&self) -> &VNode {
        self.inner.vnode_mp.get(&0).unwrap()
    }

    pub fn get_vnode(&self, id: &u64) -> Option<&VNode> {
        self.inner.vnode_mp.get(id)
    }

    pub async fn event_entry(&mut self, id: &u64, entry_name: &str, event: json::JsonValue) {
        log::debug!("event_entry: {entry_name}");
        if let Some(vnode) = self.inner.vnode_mp.get(id) {
            log::debug!("event_entry: props={}", vnode.view_props.props);
            self.load(&event, &Path::from_str("$->$:input"))
                .await
                .unwrap();
            for listener in vnode.view_props.props[entry_name].clone().members() {
                let rs = self
                    .execute_script(&vec![format!("{}", listener.as_str().unwrap())])
                    .await
                    .unwrap();
                log::debug!("{:?}", rs);
            }
        }
    }

    pub fn get_vnode_mp(&self) -> &HashMap<u64, VNode> {
        &self.inner.vnode_mp
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
