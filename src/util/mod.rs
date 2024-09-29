use edge_lib::util::engine::EdgeEngine;

use super::ViewProps;

mod inner {
    use crate::ViewProps;

    pub fn parse_child(root: &json::JsonValue, child_v: &Vec<ViewProps>) -> Vec<ViewProps> {
        if root.is_string() && root.as_str().unwrap() == "$child" {
            return child_v.clone();
        }
        vec![ViewProps {
            class: root["$:class"][0].as_str().unwrap().to_string(),
            props: root["$:props"][0].clone(),
            child_v: root["$:child"]
                .members()
                .into_iter()
                .map(|child| parse_child(child, child_v))
                .reduce(|mut acc, item| {
                    acc.extend(item);
                    acc
                })
                .unwrap(),
        }]
    }
}

pub async fn execute_as_node(
    script: &Vec<String>,
    child_v: &Vec<ViewProps>,
    mut edge_engine: EdgeEngine,
) -> ViewProps {
    let rs = edge_engine.execute_script(script).await.unwrap();
    let root_v = json::parse(&edge_lib::util::rs_2_str(&rs)).unwrap();
    inner::parse_child(&root_v[0], child_v).pop().unwrap()
}
