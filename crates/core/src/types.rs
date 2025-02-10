pub type IModelsQueryBindValueTuple = (String, String);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IResult<T> {
    pub result: T,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IResultList<T> {
    pub results: Vec<T>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IResultPass {
    pub pass: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IError {
    pub err: String,
}
