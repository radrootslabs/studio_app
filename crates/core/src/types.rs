pub type IModelsQueryBindValue = String;
pub type IModelsQueryBindValueTuple = (String, IModelsQueryBindValue);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IModelsId {
    pub id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IModelsResults<T> {
    pub results: Vec<T>,
}
