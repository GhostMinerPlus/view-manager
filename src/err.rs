use std::fmt::Debug;

#[derive(Debug)]
pub struct Error<K>
where
    K: Debug,
{
    kind: K,
    message: String,
    stack_v: Vec<String>,
}

impl<K> Error<K>
where
    K: Debug,
{
    pub fn new(kind: K, message: String) -> Self {
        Self {
            kind,
            message,
            stack_v: vec![],
        }
    }

    pub fn append_stack(mut self, stack: String) -> Self {
        self.stack_v.push(stack);
        self
    }

    pub fn kind(&self) -> &K {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Other,
    NotFound,
}

pub type Result<T> = std::result::Result<T, Error<ErrorKind>>;
