use uuid::Uuid;

pub fn new_id() -> Uuid { Uuid::now_v7() }
