use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Response<T> {
    Ok(T),
    Err(String),
}

impl<T> Response<T>
where
    T: for<'de> Deserialize<'de>,
{
    pub fn new_ok(data: T) -> Self {
        Self::Ok(data)
    }
}
