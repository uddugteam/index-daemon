use uuid::Uuid;

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ConnectionId(String);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}
