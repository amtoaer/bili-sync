use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upper<T> {
    pub mid: T,
    pub name: String,
    pub face: String,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct UpperVec(pub Vec<Upper<i64>>);

impl From<Vec<Upper<i64>>> for UpperVec {
    fn from(value: Vec<Upper<i64>>) -> Self {
        Self(value)
    }
}

impl From<UpperVec> for Vec<Upper<i64>> {
    fn from(value: UpperVec) -> Self {
        value.0
    }
}
