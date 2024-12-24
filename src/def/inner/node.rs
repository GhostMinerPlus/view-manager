use moon_class::executor::{def::AsClassManagerHolder, ClassExecutor};

use crate::{bean::ViewProps, def::AsViewManager};

mod inner {
    use error_stack::ResultExt;
    use moon_class::err;

    use crate::bean::ViewProps;

    use super::Node;

    pub fn parse_child(root: &json::JsonValue) -> Node<ViewProps> {
        if root.is_string() && root.as_str().unwrap() == "$child" {
            return Node::new(ViewProps {
                class: format!("$child"),
                props: json::Null,
            });
        }
        Node::new_with_child_v(
            ViewProps {
                class: root["$class"][0]
                    .as_str()
                    .ok_or(err::Error::RuntimeError)
                    .attach_printable_lazy(|| {
                        format!("root = {root}, root[$class] = {}", root["$class"])
                    })
                    .unwrap()
                    .to_string(),
                props: root["$props"][0].clone(),
            },
            root["$child"]
                .members()
                .into_iter()
                .map(|child| parse_child(child))
                .collect(),
        )
    }
}

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

pub async fn execute_as_node(script: String, vm: &mut impl AsViewManager) -> Node<ViewProps> {
    log::debug!("execute_as_node: script = {script}");

    let mut ce = ClassExecutor::new(vm);

    let rs = ce.execute_script(&script).await.unwrap();

    log::debug!("execute_as_node: root = {}", rs[0]);

    let root = ce.dump(&rs[0]).await;

    log::debug!("execute_as_node: {root}");

    inner::parse_child(&root)
}
