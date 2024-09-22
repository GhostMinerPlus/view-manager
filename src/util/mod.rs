use edge_lib::engine::{EdgeEngine, ScriptTree1};

use super::ViewProps;

mod inner {
    use crate::ViewProps;

    pub fn slice(rs: &json::JsonValue, step_v: &[&str], index_v: &[usize]) -> json::JsonValue {
        let mut new_rs = json::object! {};
        let mut root = rs;
        for step in step_v {
            root = &root[*step];
        }
        pass_path(root, index_v, &mut new_rs);
        new_rs
    }

    pub fn pass_path(rs: &json::JsonValue, index_v: &[usize], new_rs: &mut json::JsonValue) {
        if rs.is_array() {
            let mut root = rs;
            for index in index_v {
                root = &root[*index];
            }
            *new_rs = root.clone();
            return;
        }
        for (step, value) in rs.entries() {
            let mut new_sub_rs = json::object! {};
            pass_path(value, index_v, &mut new_sub_rs);
            let _ = new_rs.insert(step, new_sub_rs);
        }
    }

    pub fn parse_child(rs: &json::JsonValue) -> Vec<ViewProps> {
        let mut child_v = Vec::new();
        for i in 0..rs["child"]["class"][0].len() {
            let class = rs["child"]["class"][i][0].as_str().unwrap().to_string();
            let props = slice(&rs, &["child", "props"], &[i, 0]);
            child_v.push(ViewProps {
                class,
                props,
                child_v: parse_child(&slice(&rs, &["child"], &[i])),
            });
        }
        child_v
    }
}

pub async fn execute_as_node(
    script: &ScriptTree1,
    mut edge_engine: EdgeEngine,
) -> Option<ViewProps> {
    let rs = edge_engine.execute2(script).await.unwrap();
    inner::parse_child(&rs).pop()
}
