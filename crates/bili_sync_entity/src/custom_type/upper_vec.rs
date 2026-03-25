use std::borrow::Cow;

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

impl<T, S: AsRef<str>> Upper<T, S> {
    pub fn role(&self) -> Cow<'_, str> {
        if let Some(title) = &self.title {
            Cow::Owned(format!("{}「{}」", self.name.as_ref(), title.as_ref()))
        } else {
            Cow::Borrowed(self.name.as_ref())
        }
    }
}
