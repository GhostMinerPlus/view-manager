//! A view manager, let all types of layout be as html.

use std::pin::Pin;

use moon_class::{util::rs_2_str, AsClassManager, Fu};

mod node;
mod inner {
    use std::pin::Pin;

    use error_stack::ResultExt;
    use moon_class::{
        util::executor::{ClassExecutor, ClassManagerHolder},
        Fu,
    };

    use crate::{err, node::Node};

    use super::{AsViewManager, ViewProps};

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
        inner_id: u64,
        view_props_node: &'a1 Node<ViewProps>,
        embeded_id: u64,
    ) -> Pin<Box<dyn Fu<Output = ()> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
    {
        Box::pin(async move {
            for i in 0..view_props_node.child_v.len() {
                let child_id = match vm.get_vnode(&inner_id).unwrap().embeded_child_v.get(i) {
                    Some(r) => *r,
                    None => {
                        let new_id = vm.new_vnode(context);
                        vm.get_vnode_mut(&inner_id)
                            .unwrap()
                            .embeded_child_v
                            .push(new_id);
                        new_id
                    }
                };

                apply_inner_props_node(
                    vm,
                    context,
                    child_id,
                    &view_props_node.child_v[i],
                    embeded_id,
                )
                .await;
            }

            trunc_embeded(inner_id, vm, view_props_node.child_v.len());

            vm.apply_props(inner_id, &view_props_node.data, inner_id, false)
                .await
                .unwrap();
        })
    }
}

pub mod err;

#[derive(PartialEq, Clone, Debug, Eq)]
pub struct ViewProps {
    pub class: String,
    pub props: json::JsonValue,
}

impl Ord for ViewProps {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for ViewProps {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.class.partial_cmp(&other.class) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.props.to_string().partial_cmp(&other.props.to_string())
    }
}

#[derive(Clone)]
pub struct VNode {
    pub view_props: ViewProps,
    pub state: json::JsonValue,
    pub embeded_child_v: Vec<u64>,
    pub inner_node: node::Node<u64>,
    pub context: u64,
}

impl VNode {
    pub fn new(context: u64) -> Self {
        Self {
            view_props: ViewProps {
                class: String::new(),
                props: json::Null,
            },
            state: json::object! {},
            embeded_child_v: vec![],
            inner_node: node::Node::new(0),
            context,
        }
    }
}

pub trait AsViewManager: AsClassManager + AsElementProvider<H = u64> {
    fn on_update_vnode_props(&mut self, id: u64, props: &ViewProps) {
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

                    self.get_vnode_mut(&context).unwrap().state = n_state;
                    self.apply_props(
                        context,
                        &self.get_vnode(&context).unwrap().view_props.clone(),
                        self.get_vnode(&context).unwrap().context,
                        true,
                    )
                    .await
                    .unwrap();
                }
            }
            Ok(())
        })
    }

    fn apply_props<'a, 'a1, 'f>(
        &'a mut self,
        vnode_id: u64,
        view_props: &'a1 ViewProps,
        embeded_id: u64,
        force: bool,
    ) -> Pin<Box<dyn Fu<Output = err::Result<()>> + 'f>>
    where
        'a: 'f,
        'a1: 'f,
        Self: Sized,
    {
        Box::pin(async move {
            if view_props.class == "$child" {
                self.get_vnode_mut(&vnode_id).unwrap().embeded_child_v = self
                    .get_vnode(&embeded_id)
                    .unwrap()
                    .inner_node
                    .child_v
                    .iter()
                    .map(|node| node.data)
                    .collect();

                return Ok(());
            }

            if !force && self.get_vnode(&vnode_id).unwrap().view_props == *view_props {
                return Ok(());
            }

            self.on_update_vnode_props(vnode_id, view_props);

            self.get_vnode_mut(&vnode_id).unwrap().view_props = view_props.clone();

            if let Some(inner_props_node) = inner::layout(self, vnode_id, &view_props).await? {
                if self.get_vnode(&vnode_id).unwrap().inner_node.data == 0 {
                    self.get_vnode_mut(&vnode_id).unwrap().inner_node =
                        node::Node::new(self.new_vnode(vnode_id));
                }

                let inner_id = self.get_vnode(&vnode_id).unwrap().inner_node.data;

                inner::apply_inner_props_node(
                    self,
                    vnode_id,
                    inner_id,
                    &inner_props_node,
                    embeded_id,
                )
                .await;
            } else if self.get_vnode(&vnode_id).unwrap().inner_node.data != 0 {
                let inner_id = self.get_vnode(&vnode_id).unwrap().inner_node.data;
                inner::trunc_embeded(inner_id, self, 0);
                self.rm_vnode(inner_id);
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

    fn new_vnode(&mut self, context: u64) -> u64;

    fn rm_vnode(&mut self, id: u64) -> Option<VNode>;
}

pub trait AsElementProvider {
    type H;

    fn update_element(&mut self, id: Self::H, class: &str, props: &json::JsonValue);

    fn delete_element(&mut self, id: Self::H);

    fn create_element(&mut self, vnode_id: u64, class: &str, props: &json::JsonValue) -> Self::H;
}
