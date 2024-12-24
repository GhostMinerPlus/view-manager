use std::{collections::BTreeMap, pin::Pin};

use error_stack::ResultExt;
use moon_class::{def::Fu, executor::ClassExecutor};

use crate::{
    bean::{VNode, ViewProps},
    err,
};

use super::AsViewManager;

mod node;

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
) -> err::Result<Option<node::Node<ViewProps>>> {
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

        Some(node::execute_as_node(format!("{pre_script}{script}"), vm).await)
    } else {
        None
    };

    Ok(rs)
}

pub async fn event_handler(
    vm: &mut impl AsViewManager,
    data: &json::JsonValue,
    vnode_id: u64,
    script: String,
) -> err::Result<()> {
    let pre_script = format!(
        r#"
{data} = $data();
{vnode_id} = $vnode_id();
"#
    );

    let script = format!("{pre_script}{script}");

    log::debug!("event_handler: script = {script}");

    let mut ce = ClassExecutor::new(vm);

    ce.execute_script(&script)
        .await
        .change_context(err::Error::RuntimeError)?;

    Ok(())
}

pub fn apply_inner_props_node<'a, 'a1, 'f>(
    vm: &'a mut impl AsViewManager,
    context: u64,
    vnode_id: u64,
    view_props_node: &'a1 node::Node<ViewProps>,
    embeded_id: u64,
) -> Pin<Box<dyn Fu<Output = ()> + 'f>>
where
    'a: 'f,
    'a1: 'f,
{
    Box::pin(async move {
        if !view_props_node.child_v.is_empty() && view_props_node.child_v[0].data.class == "$child"
        {
            trunc_embeded(vnode_id, vm, 0);

            let embeded_child_v = vm.get_vnode(&embeded_id).unwrap().embeded_child_v.clone();

            for id in &embeded_child_v {
                vm.get_vnode_mut(id).unwrap().parent_op = Some(vnode_id);
            }

            vm.get_vnode_mut(&vnode_id).unwrap().embeded_child_v = embeded_child_v;
        } else {
            let node_type = view_props_node.data.props["$type"][0]
                .as_str()
                .unwrap_or("list");

            match node_type {
                "set" => {
                    let mut embeded_child_mp = BTreeMap::new();

                    for id in &vm.get_vnode(&vnode_id).unwrap().embeded_child_v {
                        embeded_child_mp.insert(vm.get_vnode(id).unwrap().view_props.clone(), *id);
                    }

                    for node in &view_props_node.child_v {
                        match embeded_child_mp.remove(&node.data) {
                            Some(id) => {
                                apply_inner_props_node(vm, context, id, node, embeded_id).await
                            }
                            None => {
                                let new_id = vm.new_vnode(VNode::new(context, Some(vnode_id)));
                                vm.get_vnode_mut(&vnode_id)
                                    .unwrap()
                                    .embeded_child_v
                                    .push(new_id);

                                apply_inner_props_node(vm, context, new_id, node, embeded_id).await
                            }
                        }
                    }

                    let embeded_child_v = &mut vm.get_vnode_mut(&vnode_id).unwrap().embeded_child_v;

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
                            .map(|_| vm.new_vnode(VNode::new(context, Some(vnode_id))))
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
