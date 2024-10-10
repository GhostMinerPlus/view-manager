//!

mod inner_util;
mod inner {
    use edge_lib::util::Path;

    use super::{AsViewManager, Node, ViewProps};

    pub fn trunc_embeded(vnode_id: u64, vm: &mut impl AsViewManager, n_sz: usize) {
        let embeded_child_v = &mut vm.get_vnode_mut(&vnode_id).unwrap().embeded_child_v;
        for id in embeded_child_v.split_off(n_sz) {
            trunc_embeded(id, vm, 0);
            if vm.get_vnode(&id).unwrap().context == vnode_id {
                vm.rm_vnode(id);
            }
        }
    }

    ///
    pub async fn layout(
        vnode_id: u64,
        view_props: &ViewProps,
        vm: &mut impl AsViewManager,
    ) -> Option<Node<ViewProps>> {
        vm.load(&view_props.props, &Path::from_str("$->$:props"))
            .await
            .unwrap();
        vm.set(&Path::from_str("$->$:vnode_id"), vec![vnode_id.to_string()])
            .await
            .unwrap();
        if let Some(script) = vm.get_class(&view_props.class) {
            Some(super::inner_util::execute_as_node(&script.clone(), vm).await)
        } else {
            None
        }
    }
}

use std::{future::Future, pin::Pin};

use edge_lib::util::{
    data::{AsDataManager, AsStack},
    engine::AsEdgeEngine,
    Path,
};
use inner_util::Node;

use crate::err;

#[derive(PartialEq, Clone, Debug)]
pub struct ViewProps {
    pub class: String,
    pub props: json::JsonValue,
}

#[derive(Clone)]
pub struct VNode {
    pub view_props: ViewProps,
    pub embeded_child_v: Vec<u64>,
    pub inner_node: Node<u64>,
    pub context: u64,
}

impl VNode {
    pub fn new(context: u64) -> Self {
        Self {
            view_props: ViewProps {
                class: String::new(),
                props: json::Null,
            },
            embeded_child_v: vec![],
            inner_node: Node::new(0),
            context,
        }
    }
}

pub trait AsViewManager: AsDataManager + AsStack {
    fn get_class(&self, class: &str) -> Option<&Vec<String>>;

    fn get_vnode(&self, id: &u64) -> Option<&VNode>;

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut VNode>;

    fn new_vnode(&mut self, context: u64) -> u64;

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
            self.load(&event, &Path::from_str("$->$:event"))
                .await
                .unwrap();
            if let Some(vnode) = self.get_vnode(&id) {
                log::debug!("event_entry: props={}", vnode.view_props.props);
                let script = vnode.view_props.props[entry_name]
                    .members()
                    .map(|s| s.as_str().unwrap().to_string())
                    .collect::<Vec<String>>();
                let context = vnode.context;
                self.set(&Path::from_str("$->$:context"), vec![context.to_string()])
                    .await
                    .unwrap();
                self.set(&Path::from_str("$->$:vnode_id"), vec![id.to_string()])
                    .await
                    .unwrap();
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
        embeded_id: u64,
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

            let embeded_child_v = self.get_vnode(&embeded_id).unwrap().embeded_child_v.clone();
            self.on_update_vnode_props(vnode_id, view_props);

            self.get_vnode_mut(&vnode_id).unwrap().view_props = view_props.clone();

            if let Some(inner_props_node) = inner::layout(vnode_id, &view_props, self).await {
                if self.get_vnode(&vnode_id).unwrap().inner_node.data == 0 {
                    self.get_vnode_mut(&vnode_id).unwrap().inner_node =
                        Node::new(self.new_vnode(vnode_id));
                }

                let inner_id = self.get_vnode(&vnode_id).unwrap().inner_node.data;

                if inner_props_node.child_v.len() == 1
                    && inner_props_node.child_v.first().unwrap().data.class == "$child"
                {
                    if self.get_vnode(&inner_id).unwrap().embeded_child_v != embeded_child_v {
                        inner::trunc_embeded(inner_id, self, 0);
                        self.get_vnode_mut(&inner_id).unwrap().embeded_child_v = embeded_child_v;
                    }
                } else {
                    for i in 0..inner_props_node.child_v.len() {
                        let child_props = &inner_props_node.child_v[i];
                        if let None = self.get_vnode(&inner_id).unwrap().embeded_child_v.get(i) {
                            let new_id = self.new_vnode(vnode_id);
                            self.get_vnode_mut(&inner_id)
                                .unwrap()
                                .embeded_child_v
                                .push(new_id);
                        }

                        let child_id = self.get_vnode(&inner_id).unwrap().embeded_child_v[i];

                        self.apply_props(child_id, &child_props.data, embeded_id)
                            .await?;
                    }

                    inner::trunc_embeded(inner_id, self, inner_props_node.child_v.len());
                }

                self.apply_props(inner_id, &inner_props_node.data, inner_id)
                    .await?;
            } else if self.get_vnode(&vnode_id).unwrap().inner_node.data != 0 {
                let inner_id = self.get_vnode(&vnode_id).unwrap().inner_node.data;
                inner::trunc_embeded(inner_id, self, 0);
                self.rm_vnode(inner_id);
            }
            Ok(())
        })
    }
}
