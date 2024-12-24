use std::{collections::BTreeMap, pin::Pin};

use moon_class::{util::rs_2_str, def::{AsClassManager, Fu}};

use crate::{bean::{VNode, ViewProps}, err};

mod inner;

pub trait AsViewManager: AsClassManager + AsElementProvider<H = u64> {
    fn on_update_vnode_props(&mut self, id: u64, props: &ViewProps) {
        // Let the element be usable.
        if self.get_vnode(&id).unwrap().view_props.class != props.class {
            self.delete_element(id);

            self.create_element(id, &props.class, &props.props);
        } else if !self.reuse_element(id, &props.class, &props.props) {
            self.delete_element(id);

            self.create_element(id, &props.class, &props.props);
        }
    }

    fn event_entry<'a, 'a1, 'a2, 'a3, 'f>(
        &'a mut self,
        vnode_id: u64,
        entry_name: &'a2 str,
        data: &'a3 json::JsonValue,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        'a2: 'f,
        'a3: 'f,
        Self: Sized,
    {
        Box::pin(async move {
            log::debug!("event_entry: {entry_name}");
            if let Some(vnode) = self.get_vnode(&vnode_id) {
                let script = &vnode.view_props.props[entry_name];

                if script.is_empty() {
                    return Ok(());
                }

                let script = if script.is_array() {
                    rs_2_str(
                        &script
                            .members()
                            .map(|jv| jv.as_str().unwrap().to_string())
                            .collect::<Vec<String>>(),
                    )
                } else {
                    script.as_str().unwrap().to_string()
                };

                inner::event_handler(self, data, vnode_id, script).await?;
            }
            Ok(())
        })
    }

    fn update_state(&mut self, vnode_id: u64, n_state: json::JsonValue) {
        log::debug!("new state: {n_state} in {vnode_id}");
        let vnode = self.get_vnode_mut(&vnode_id).unwrap();

        vnode.state = n_state;
        vnode.is_dirty = true;
        self.dirty_vnode_v_mut().insert(vnode_id, None);
    }

    fn dirty_vnode_v_mut(&mut self) -> &mut BTreeMap<u64, Option<ViewProps>>;

    fn apply_props<'a, 'f>(
        &'a mut self,
        vnode_id: u64,
        view_props_op: Option<ViewProps>,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        Self: Sized,
    {
        Box::pin(async move {
            let vnode = match self.get_vnode_mut(&vnode_id) {
                Some(r) => r,
                None => {
                    return Ok(());
                }
            };

            if !vnode.is_dirty {
                return Ok(());
            } else {
                vnode.is_dirty = false;
            }

            let parent_op = vnode.parent_op;

            let view_props = if let Some(view_props) = view_props_op {
                let is_same_props = vnode.view_props == view_props;

                if is_same_props {
                    return Ok(());
                }

                self.on_update_vnode_props(vnode_id, &view_props);

                self.get_vnode_mut(&vnode_id).unwrap().view_props = view_props.clone();

                view_props
            } else {
                vnode.view_props.clone()
            };

            if let Some(inner_props_node) = inner::layout(self, vnode_id, &view_props).await? {
                if self.get_vnode(&vnode_id).unwrap().inner_id == 0 {
                    self.get_vnode_mut(&vnode_id).unwrap().inner_id =
                        self.new_vnode(VNode::new(vnode_id, parent_op));
                }

                let vnode = self.get_vnode(&vnode_id).unwrap();

                let inner_id = vnode.inner_id;
                let embeded_id = vnode.context;

                inner::apply_inner_props_node(
                    self,
                    vnode_id,
                    inner_id,
                    &inner_props_node,
                    embeded_id,
                )
                .await;
            } else if self.get_vnode(&vnode_id).unwrap().inner_id != 0 {
                let inner_id = self.get_vnode(&vnode_id).unwrap().inner_id;

                inner::remove_node(self, inner_id);

                self.get_vnode_mut(&vnode_id).unwrap().inner_id = 0;
            }

            Ok(())
        })
    }

    fn get_class_view<'a, 'a1, 'f>(
        &'a self,
        class: &'a1 str,
    ) -> Pin<Box<dyn Fu<Output = Option<String>> + 'f>>
    where
        'a: 'f,
        'a1: 'f;

    fn get_vnode(&self, id: &u64) -> Option<&VNode>;

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut VNode>;

    fn new_vnode(&mut self, vnode: VNode) -> u64;

    fn rm_vnode(&mut self, id: u64) -> Option<VNode>;
}

pub trait AsElementProvider {
    type H;

    fn reuse_element(&mut self, id: Self::H, class: &str, props: &json::JsonValue) -> bool;

    fn delete_element(&mut self, id: Self::H);

    fn create_element(&mut self, vnode_id: u64, class: &str, props: &json::JsonValue) -> Self::H;
}
