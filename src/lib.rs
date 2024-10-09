use std::{future::Future, pin::Pin};

use edge_lib::util::{
    data::{AsDataManager, AsStack},
    engine::AsEdgeEngine,
    Path,
};

mod inner {
    use edge_lib::util::Path;

    use crate::AsViewManager;

    use super::{Node, VNode, ViewProps};

    pub fn resize_child(vnode_id: u64, vm: &mut impl AsViewManager, n_sz: usize) {
        let diff = vm.get_vnode(&vnode_id).unwrap().inner_node.child_v.len() as i32 - n_sz as i32;
        if diff < 0 {
            let mut arr = Vec::with_capacity(-diff as usize);
            for _ in 0..-diff {
                arr.push(Node::new(vm.new_vnode()));
            }
            vm.get_vnode_mut(&vnode_id)
                .unwrap()
                .inner_node
                .child_v
                .extend(arr);
        } else if diff > 0 {
            for item in vm
                .get_vnode_mut(&vnode_id)
                .unwrap()
                .inner_node
                .child_v
                .split_off(n_sz)
            {
                resize_child(item.data, vm, 0);
                vm.rm_vnode(item.data);
            }
        }
    }

    ///
    pub async fn layout(view: &VNode, vm: &mut impl AsViewManager) -> Option<ViewProps> {
        vm.load(&view.view_props.props, &Path::from_str("$->$:input"))
            .await
            .unwrap();
        if let Some(script) = vm.get_class(&view.view_props.class) {
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

#[derive(PartialEq, Clone, Debug)]
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

pub trait AsViewManager: AsDataManager + AsStack {
    fn get_class(&self, class: &str) -> Option<&Vec<String>>;

    fn get_vnode(&self, id: &u64) -> Option<&VNode>;

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut VNode>;

    fn new_vnode(&mut self) -> u64;

    fn rm_vnode(&mut self, id: u64) -> Option<VNode>;

    fn on_update_vnode_props(&mut self, id: u64, props: &ViewProps);

    fn event_entry<'a, 'a1, 'f>(
        &'a mut self,
        id: u64,
        entry_name: &'a1 str,
        event: json::JsonValue,
    ) -> Pin<Box<dyn Future<Output = err::Result<()>> + Send + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        Self: Sized,
    {
        Box::pin(async move {
            log::debug!("event_entry: {entry_name}");
            self.load(&event, &Path::from_str("$->$:input"))
                .await
                .unwrap();
            if let Some(vnode) = self.get_vnode(&id) {
                log::debug!("event_entry: props={}", vnode.view_props.props);
                let script = vnode.view_props.props[entry_name]
                    .members()
                    .map(|s| s.as_str().unwrap().to_string())
                    .collect::<Vec<String>>();
                let rs = self.execute_script(&script).await.unwrap();
                log::debug!("{:?}", rs);
            }
            Ok(())
        })
    }

    fn apply_props<'a, 'a1, 'f>(
        &'a mut self,
        vnode_id: u64,
        view_props: &'a1 ViewProps,
    ) -> Pin<Box<dyn Future<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        Self: Sized,
    {
        Box::pin(async move {
            if self.get_vnode(&vnode_id).unwrap().view_props == *view_props {
                return Ok(());
            }

            self.on_update_vnode_props(vnode_id, view_props);

            self.get_vnode_mut(&vnode_id).unwrap().view_props = view_props.clone();

            if let Some(inner_props) =
                inner::layout(&self.get_vnode(&vnode_id).unwrap().clone(), self).await
            {
                if self.get_vnode(&vnode_id).unwrap().inner_node.data == 0 {
                    self.get_vnode_mut(&vnode_id).unwrap().inner_node = Node::new(self.new_vnode());
                }

                let inner_id = self.get_vnode(&vnode_id).unwrap().inner_node.data;

                self.apply_props(inner_id, &inner_props).await?;
            } else {
                // update meta element
                inner::resize_child(vnode_id, self, view_props.child_v.len());

                for i in 0..view_props.child_v.len() {
                    let child_props = &view_props.child_v[i];
                    let child_view_id =
                        self.get_vnode(&vnode_id).unwrap().inner_node.child_v[i].data;

                    self.apply_props(child_view_id, child_props).await?;
                }
            }
            Ok(())
        })
    }
}
