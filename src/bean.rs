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
    pub inner_id: u64,
    pub embeded_child_v: Vec<u64>,
    pub context: u64,
    pub is_dirty: bool,
    pub parent_op: Option<u64>,
}

impl VNode {
    pub fn new(context: u64, parent_op: Option<u64>) -> Self {
        Self {
            view_props: ViewProps {
                class: String::new(),
                props: json::Null,
            },
            state: json::object! {},
            inner_id: 0,
            embeded_child_v: vec![],
            context,
            is_dirty: true,
            parent_op,
        }
    }
}
