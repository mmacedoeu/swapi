use futures::{Future,future};
use actix_web::{HttpRequest, HttpResponse, HttpMessage, Error, AsyncResponder, Query};
use domain::{Planet, ReadPlanets, DeletePlanet, SearchPlanet, GetPlanet};
use actors::getfilms;
use uuid::Uuid;

#[cfg_attr(feature="flame_it", flame)]
pub fn create(req: HttpRequest<::actors::State>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    let db = {req.state().db.clone()};
    let cache = {req.state().cache.clone()};
    let processor = {req.state().processor.clone()};
    req.json()
        .from_err()
        .and_then(move |p : Planet|  {
            let name = p.name.clone();

            let _ = getfilms(&name, processor, cache, |_| ());

            db.send(p)
                .from_err()
                .and_then(|res| {
                    match res {
                        Ok(tx) => {                            
                            let map = json!({
                                  "tx_id": tx.tx_id,
                                  "tx_instant": tx.tx_instant,
                            });
                            Ok(HttpResponse::Ok().json(map))
                        } // <- send response
                        Err(e) => {
                            warn!("error: {:?}", e);
                            Ok(HttpResponse::InternalServerError().into())
                        }
                    }
                })            
        })
        .responder()
}

#[cfg_attr(feature="flame_it", flame)]
pub fn read(req: HttpRequest<::actors::State>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    let read = {req.state().read.clone()};
    Box::new(read.send(ReadPlanets{})
                .from_err()
                .and_then(|res| {
                    match res {
                        Ok(out) => {                            
                            Ok(HttpResponse::Ok().json(out))
                        } // <- send response
                        Err(e) => {
                            warn!("error: {:?}", e);
                            Ok(HttpResponse::InternalServerError().into())
                        }
                    }
                })  
    )   
}

#[cfg_attr(feature="flame_it", flame)]
pub fn delete(req: HttpRequest<::actors::State>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    let db = {req.state().db.clone()};
    let uuid = {
        if let Some(uuid) = req.match_info().get("uuid") {
            match Uuid::parse_str(uuid) {
                Ok(u) => u,
                Err(_) => return Box::new(future::ok(HttpResponse::InternalServerError().into())),
            }
        } else {
            return Box::new(future::ok(HttpResponse::InternalServerError().into()));
        }
    };

    debug!("got uuid: \t {}", uuid);

    Box::new(db.send(DeletePlanet(uuid))
        .from_err()
        .and_then(|res| {
            match res {
                Ok(tx) => {                      
                    let map = json!({
                        "tx_id": tx.tx_id,
                        "tx_instant": tx.tx_instant,
                    });
                    Ok(HttpResponse::Ok().json(map))
                } // <- send response
                Err(e) => {
                    warn!("error: {:?}", e);
                    Ok(HttpResponse::InternalServerError().into())
                }
            }
        }))
}

#[derive(Deserialize)]
pub struct SearchParam {
    pub search : String,
}

#[cfg_attr(feature="flame_it", flame)]
pub fn search(req: HttpRequest<::actors::State>, info: Query<SearchParam>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    let read = {req.state().read.clone()};
    debug!("got search: \t {}", info.search);
    Box::new(read.send(SearchPlanet(info.search.clone()))
                .from_err()
                .and_then(|res| {
                    match res {
                        Ok(out) => {                            
                            Ok(HttpResponse::Ok().json(out))
                        } // <- send response
                        Err(e) => {
                            warn!("error: {:?}", e);
                            Ok(HttpResponse::InternalServerError().into())
                        }
                    }
                })  
    )   
}

#[cfg_attr(feature="flame_it", flame)]
pub fn id(req: HttpRequest<::actors::State>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    let read = {req.state().read.clone()};
    let uuid = {
        if let Some(uuid) = req.match_info().get("uuid") {
            match Uuid::parse_str(uuid) {
                Ok(u) => u,
                Err(_) => return Box::new(future::ok(HttpResponse::InternalServerError().into())),
            }
        } else {
            return Box::new(future::ok(HttpResponse::InternalServerError().into()));
        }
    };

    debug!("got uuid: \t {}", uuid); 

    Box::new(read.send(GetPlanet(uuid))
                .from_err()
                .and_then(|res| {
                    match res {
                        Ok(out) => {                            
                            Ok(HttpResponse::Ok().json(out))
                        } // <- send response
                        Err(e) => {
                            warn!("error: {:?}", e);
                            Ok(HttpResponse::InternalServerError().into())
                        }
                    }
                })  
    )         
}