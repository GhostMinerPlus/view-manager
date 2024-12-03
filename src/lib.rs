//! A view manager, let all types of layout be as html.

use std::{collections::BTreeMap, pin::Pin};

use moon_class::{util::rs_2_str, AsClassManager, Fu};

mod node;
mod inner {
    use std::{collections::BTreeMap, pin::Pin};

    use error_stack::ResultExt;
    use moon_class::{
        util::executor::{ClassExecutor, ClassManagerHolder},
        Fu,
    };

    use crate::{bean::ViewProps, err, node::Node};

    use super::AsViewManager;

    pub fn trunc_embeded(vnode_id: u64, vm: &mut impl AsViewManager, n_sz: usize) {
        let embeded_child_v = &mut match vm.get_vnode_mut(&vnode_id) {
            Some(r) => r,
            None => {
                return;
            }
        }
        .embeded_child_v;

        for id in embeded_child_v.split_off(n_sz) {
            remove_node(vm, id);
        }
    }

    pub fn remove_node(vm: &mut impl AsViewManager, id: u64) {
        trunc_embeded(id, vm, 0);

        let inner_id = match vm.get_vnode(&id) {
            Some(r) => r,
            None => {
                return;
            }
        }
        .inner_id;

        if inner_id != 0 {
            remove_node(vm, inner_id);
        }

        vm.delete_element(id);
        vm.rm_vnode(id);
    }

    ///
    pub async fn layout(
        vm: &mut impl AsViewManager,
        vnode_id: u64,
        view_props: &ViewProps,
    ) -> err::Result<Option<Node<ViewProps>>> {
        let rs = if let Some(script) = vm.get_class_view(&view_props.class).await {
            let state = vm
                .get_vnode(&vnode_id)
                .ok_or(err::Error::NotFound)
                .attach_printable_lazy(|| format!("vnode with id {vnode_id} not found!"))?
                .state
                .clone();

            let pre_script = format!(
                r#"{state} = $state();
{} = $props();
{vnode_id} = $vnode_id();
"#,
                view_props.props
            );

            Some(super::node::execute_as_node(format!("{pre_script}{script}"), vm).await)
        } else {
            None
        };

        Ok(rs)
    }

    pub async fn event_handler(
        vm: &mut impl AsViewManager,
        data: &json::JsonValue,
        context: u64,
        vnode_id: u64,
        state: &json::JsonValue,
        script: String,
    ) -> err::Result<json::JsonValue> {
        let pre_script = format!(
            r#"
{data} = $data();
{state} = $state();
{context} = $context();
{vnode_id} = $vnode_id();
"#
        );

        let script = format!("{pre_script}{script}");

        log::debug!("event_handler: script = {script}");

        let mut ce = ClassExecutor::new(vm);

        let rs = ce
            .execute_script(&script)
            .await
            .change_context(err::Error::RuntimeError)?;

        Ok(if rs.is_empty() {
            json::Null
        } else {
            ce.temp_ref().dump(&rs[0])
        })
    }

    pub fn apply_inner_props_node<'a, 'a1, 'f>(
        vm: &'a mut impl AsViewManager,
        context: u64,
        vnode_id: u64,
        view_props_node: &'a1 Node<ViewProps>,
        embeded_id: u64,
    ) -> Pin<Box<dyn Fu<Output = ()> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            if !view_props_node.child_v.is_empty()
                && view_props_node.child_v[0].data.class == "$child"
            {
                trunc_embeded(vnode_id, vm, 0);

                vm.get_vnode_mut(&vnode_id).unwrap().embeded_child_v =
                    vm.get_vnode(&embeded_id).unwrap().embeded_child_v.clone();
            } else {
                let node_type = view_props_node.data.props["$type"][0]
                    .as_str()
                    .unwrap_or("list");

                match node_type {
                    "set" => {
                        let mut embeded_child_mp = BTreeMap::new();

                        for id in &vm.get_vnode(&vnode_id).unwrap().embeded_child_v {
                            embeded_child_mp
                                .insert(vm.get_vnode(id).unwrap().view_props.clone(), *id);
                        }

                        for node in &view_props_node.child_v {
                            match embeded_child_mp.remove(&node.data) {
                                Some(id) => {
                                    apply_inner_props_node(vm, context, id, node, embeded_id).await
                                }
                                None => {
                                    let new_id = vm.new_vnode(context);
                                    vm.get_vnode_mut(&vnode_id)
                                        .unwrap()
                                        .embeded_child_v
                                        .push(new_id);

                                    apply_inner_props_node(vm, context, new_id, node, embeded_id)
                                        .await
                                }
                            }
                        }

                        let embeded_child_v =
                            &mut vm.get_vnode_mut(&vnode_id).unwrap().embeded_child_v;

                        embeded_child_v.sort();

                        for (_, id) in &embeded_child_mp {
                            let index = embeded_child_v.binary_search(id).unwrap();
                            embeded_child_v.remove(index);
                        }

                        for (_, id) in &embeded_child_mp {
                            remove_node(vm, *id);
                        }
                    }
                    "list" => {
                        let diff = view_props_node.child_v.len() as i32
                            - vm.get_vnode(&vnode_id).unwrap().embeded_child_v.len() as i32;
                        if diff > 0 {
                            let new_id_v = (0..diff)
                                .into_iter()
                                .map(|_| vm.new_vnode(context))
                                .collect::<Vec<u64>>();

                            vm.get_vnode_mut(&vnode_id)
                                .unwrap()
                                .embeded_child_v
                                .extend(new_id_v);
                        } else {
                            trunc_embeded(vnode_id, vm, view_props_node.child_v.len());
                        }

                        for i in 0..view_props_node.child_v.len() {
                            let child_id = *vm
                                .get_vnode(&vnode_id)
                                .unwrap()
                                .embeded_child_v
                                .get(i)
                                .unwrap();

                            apply_inner_props_node(
                                vm,
                                context,
                                child_id,
                                &view_props_node.child_v[i],
                                embeded_id,
                            )
                            .await;
                        }
                    }
                    _ => todo!(),
                }
            }

            let vnode = vm.get_vnode_mut(&vnode_id).unwrap();

            if vnode.view_props != view_props_node.data {
                vnode.is_dirty = true;
                vm.dirty_vnode_v_mut()
                    .insert(vnode_id, Some(view_props_node.data.clone()));
            }
        })
    }
}

pub mod bean;
pub mod err;

pub trait AsViewManager: AsClassManager + AsElementProvider<H = u64> {
    fn on_update_vnode_props(&mut self, id: u64, props: &bean::ViewProps) {
        // Let the element be usable.
        if self.get_vnode(&id).unwrap().view_props.class != props.class {
            self.delete_element(id);

            self.create_element(id, &props.class, &props.props);
        } else {
            // Let the element be updated.
            self.update_element(id, &props.class, &props.props);
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

                let context = vnode.context;

                let state = self.get_vnode(&context).unwrap().state.clone();

                let rs = inner::event_handler(self, data, context, vnode_id, &state, script).await;

                let n_state = rs?;

                if !n_state.is_null() && n_state != state {
                    log::debug!("new state: {n_state} in {context}");
                    let vnode = self.get_vnode_mut(&context).unwrap();

                    vnode.state = n_state;
                    vnode.is_dirty = true;
                    self.dirty_vnode_v_mut().insert(context, None);
                }
            }
            Ok(())
        })
    }

    fn dirty_vnode_v_mut(&mut self) -> &mut BTreeMap<u64, Option<bean::ViewProps>>;

    fn apply_props<'a, 'f>(
        &'a mut self,
        vnode_id: u64,
        view_props_op: Option<bean::ViewProps>,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        Self: Sized,
    {
        Box::pin(async move {
            let vnode = self.get_vnode_mut(&vnode_id).unwrap();

            if !vnode.is_dirty {
                return Ok(());
            } else {
                vnode.is_dirty = false;
            }

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
                    self.get_vnode_mut(&vnode_id).unwrap().inner_id = self.new_vnode(vnode_id);
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

    fn get_vnode(&self, id: &u64) -> Option<&bean::VNode>;

    fn get_vnode_mut(&mut self, id: &u64) -> Option<&mut bean::VNode>;

    fn new_vnode(&mut self, context: u64) -> u64;

    fn rm_vnode(&mut self, id: u64) -> Option<bean::VNode>;
}

pub trait AsElementProvider {
    type H;

    fn update_element(&mut self, id: Self::H, class: &str, props: &json::JsonValue);

    fn delete_element(&mut self, id: Self::H);

    fn create_element(&mut self, vnode_id: u64, class: &str, props: &json::JsonValue) -> Self::H;
}
