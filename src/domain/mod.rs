use actix::{Message,Syn,dev::Request};
use errors::Result;
use mentat::TxReport;
use serde_json::value::Value;
use crossbeam_channel::Receiver;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Planet {
    pub name: String,
    pub climate: String,
    pub terrain: String,
}

pub struct InnerPlanet {
    pub uuid: String,
    pub name: ::std::sync::Arc<String>,
    pub climate: ::std::sync::Arc<String>,
    pub terrain: ::std::sync::Arc<String>,
    pub request: Option<Request<Syn, ::actors::FilmsExecutor, ReadFilms>>,
}

impl Message for Planet {
    type Result = Result<TxReport>;
}

pub struct SearchResponse(pub String, pub Vec<u8>);

impl Message for SearchResponse {
    type Result = Result<i64>;
}

pub struct ReadPlanets;

impl Message for ReadPlanets {
    type Result = Result<Value>;
}

pub struct ReadFilms(pub String);

impl Message for ReadFilms {
    type Result = Result<Receiver<Result<i64>>>;
}

pub struct DeletePlanet(pub Uuid);

impl Message for DeletePlanet {
    type Result = Result<TxReport>;
}

pub struct GetPlanet(pub Uuid);

impl Message for GetPlanet {
    type Result = Result<Value>;
}

pub struct SearchPlanet(pub String);

impl Message for SearchPlanet {
    type Result = Result<Value>;
}


