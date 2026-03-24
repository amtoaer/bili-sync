use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upper<T, S> {
    pub mid: T,
    pub name: S,
    pub face: S,
    pub title: Option<S>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct UpperVec(pub Vec<Upper<i64, String>>);

impl From<Vec<Upper<i64, String>>> for UpperVec {
    fn from(value: Vec<Upper<i64, String>>) -> Self {
        Self(value)
    }
}

impl From<UpperVec> for Vec<Upper<i64, String>> {
    fn from(value: UpperVec) -> Self {
        value.0
    }
}

impl<T: Copy> Upper<T, String> {
    pub fn as_ref(&self) -> Upper<T, &str> {
        Upper {
            mid: self.mid,
            name: self.name.as_str(),
            face: self.face.as_str(),
            title: self.title.as_deref(),
        }
    }
}
