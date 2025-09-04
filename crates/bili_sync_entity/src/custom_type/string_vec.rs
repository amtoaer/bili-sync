use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

// reference: https://www.sea-ql.org/SeaORM/docs/generate-entity/column-types/#json-column
// 在 entity 中使用裸 Vec 仅在 postgres 中支持，sea-orm 会将其映射为 postgres array
// 如果需要实现跨数据库的 array，必须将其包裹在 wrapper type 中
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct StringVec(pub Vec<String>);

impl From<Vec<String>> for StringVec {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}

impl From<StringVec> for Vec<String> {
    fn from(value: StringVec) -> Self {
        value.0
    }
}
