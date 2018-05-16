use pikkr::Pikkr;
use actix::{Syn, Addr, Actor, SyncContext, Context, Handler, Arbiter, Supervised};
use domain::{Planet, InnerPlanet, SearchResponse, ReadPlanets
    , ReadFilms, DeletePlanet, GetPlanet, SearchPlanet};
use lru_time_cache::LruCache;
use std::sync::{Mutex, Arc};
use std::clone::Clone;
use errors::{Result, Error, ErrorKind};
use mentat::{Store,TxReport, TypedValue, entity_builder::{BuildTerms,TermBuilder}
    , QueryBuilder,Queryable, IntoResult, QueryInputs,KnownEntid, Binding};
use serde_json::value::Value;
use uuid::Uuid;
use futures::{Future, Stream};
use crossbeam_channel::{bounded,Receiver};
use actix_web::client;
use actix::registry::ArbiterService;
use std::ops::Deref;

pub struct State {
    pub db: Addr<Syn, WriterExecutor>,
    pub processor: Addr<Syn, InboundCacheProcessor<'static>>,
    pub read: Addr<Syn, ReadExecutor>,
    pub cache: Arc<Mutex<LruCache<String, i64>>>,
}

pub struct WriterExecutor {
    pub store: Store,
}

impl Actor for WriterExecutor {
    type Context = SyncContext<Self>;
}

impl Handler<Planet> for WriterExecutor {
    type Result = Result<TxReport>;

    #[cfg_attr(feature="flame_it", flame)]
    fn handle(&mut self, msg: Planet, _: &mut Self::Context) -> Self::Result {
        let ip = self.store.begin_transaction()?;
        let mut builder = ip.builder().describe_tempid("x");;

        let uuid = Uuid::new_v4();
        let v_uuid = TypedValue::from(uuid);
        let v_name = TypedValue::from(msg.name);
        let v_climate = TypedValue::from(msg.climate);
        let v_terrain = TypedValue::from(msg.terrain);

        builder.add_kw(&kw!(:planet/uuid), v_uuid)?;
        builder.add_kw(&kw!(:planet/name), v_name)?;
        builder.add_kw(&kw!(:planet/climate), v_climate)?;
        builder.add_kw(&kw!(:planet/terrain), v_terrain)?;

        builder.commit().map_err(Into::into)
    }
}

fn eav(row: Vec<Binding>) -> (KnownEntid, KnownEntid, TypedValue) {
    let mut row = row.into_iter();
    match (row.next(), row.next(), row.next()) {
        (Some(Binding::Scalar(TypedValue::Ref(e))), Some(Binding::Scalar(TypedValue::Ref(a))), Some(Binding::Scalar(v))) => {
            (KnownEntid(e), KnownEntid(a), v)
        },
        _ => panic!("Incorrect query shape for 'eav' helper."),
    }
}

impl Handler<DeletePlanet> for WriterExecutor {
    type Result = Result<TxReport>;

    #[cfg_attr(feature="flame_it", flame)]
    fn handle(&mut self, msg: DeletePlanet, _: &mut Self::Context) -> Self::Result {
        let mut ip = self.store.begin_transaction()?;
        let mut builder = TermBuilder::new();
        for (e, a, v) in ip.q_once("[:find ?e ?a ?v
                                  :in ?id
                                  :where [?e ?a ?v][?e :planet/uuid ?id]]",
                                 QueryInputs::with_value_sequence(vec![(var!(?id), msg.0.into())]))
                         .into_rel_result()?
                         .into_iter()
                         .map(eav) {
            debug!("got :\t {:?} \n {:?} \n {:?}", e, a, v);
            builder.retract(e, a, v.clone())?;
        }

        let res = ip.transact_builder(builder).map_err(Into::into);
        let _ = ip.commit()?;
        res
    }
}    

pub struct FilmsExecutor {
    pub processor: Option<Addr<Syn, InboundCacheProcessor<'static>>>,
    pub cache: Option<Arc<Mutex<LruCache<String, i64>>>>
}

impl Actor for FilmsExecutor {
    type Context = Context<Self>;
}

impl Supervised for FilmsExecutor {}

impl ArbiterService for FilmsExecutor {}

impl Default for FilmsExecutor {
    fn default() -> FilmsExecutor {
        FilmsExecutor {
            processor: None,
            cache: None,
        }
    }
}

impl Handler<ReadFilms> for FilmsExecutor {
    type Result = Result<Receiver<Result<i64>>>;

    #[cfg_attr(feature="flame_it", flame)]
    fn handle(&mut self, msg: ReadFilms, _: &mut Self::Context) -> Self::Result {
        let c = {self.cache.clone()};
        let p = {self.processor.clone()};        
        let (tx, rx) = bounded::<Result<i64>>(1);
        let s = tx.clone(); 
        if let Some(cache) = c {
            if let Some(proc) = p {
                getfilms(&msg.0, proc, cache, move |result| {
                    debug!("got result in FilmsExecutor: \t {:?}", result);                    
                    match s.send(result) {
                        Ok(_) => trace!("result sent in FilmsExecutor"),
                        Err(e) => warn!("got error: 4\t {}", e),
                    }
                }); 
            }
        }
 
        debug!("return rx in FilmsExecutor");  
        Ok(rx)
    }
}

#[cfg_attr(feature="flame_it", flame)]
pub fn getfilms<F> (name: &str, processor : Addr<Syn, InboundCacheProcessor<'static>>, 
                cache : Arc<Mutex<LruCache<String, i64>>>, f: F) where
                F : Fn(Result<i64>) + 'static
{
        {
            match cache.lock() {
                Ok(mut guard) => {
                    trace!("cache len: {}", guard.len());
                    if let Some(v) = guard.get(name) {
                        debug!("Cache hit");
                        f(Ok((*v).to_owned()));
                        return;
                    }
                }                    
                Err(poisoned) => {
                    f(Err(Error::from_kind(
                            ErrorKind::Poisoned(
                                String::from(format!("{}", poisoned))
                            )
                        )
                    )); 
                    return;
                }
            }                
        }

        let farc = Arc::new(f);
        let farc1 = farc.clone();
        let farc2 = farc.clone();
        let farc3 = farc.clone();
        let farc4 = farc.clone();
        let name_string = String::from(name);
        let fut = {
            let url = format!("https://swapi.co/api/planets/?search={}",name);
            client::get(url)   // <- Create request builder
                .header("User-Agent", "Actix-web")
                .finish().unwrap()
                .send()                               // <- Send http request
                .map_err(move |e| {
                    warn!("got error 1: \t {}", e);
                    &farc1(Err(Error::from_kind(ErrorKind::Failure(::failure::Error::from(e).compat()))));
                })
                .and_then(|response|                 // <- server http response
                    response.concat2()     // <- get Body future
                            .map_err(move |e| {
                                warn!("got error 2: \t {}", e);
                                &farc2(Err(Error::from_kind(ErrorKind::Failure(::failure::Error::from(e).compat()))));
                            })
                            .and_then(move |body| {  // <- complete body                              
                                processor.send(SearchResponse(name_string,body.to_vec()))
                                    .map_err(move |e| {
                                        warn!("got error 3: \t {}", e);
                                        &farc3(Err(Error::from_kind(ErrorKind::Failure(::failure::Error::from(e).compat()))));
                                    })
                                    .and_then(move |qtdres| {
                                        debug!("got qtdres \t {:?}", qtdres);
                                        &farc4(qtdres);
                                        Ok(())
                                    })
                            })
                )

        };

        Arbiter::handle().spawn(fut.map_err(|_|()).map(|_|()));
}

pub struct InboundCacheProcessor <'a> {
    pub pikkr: Pikkr<'a>, 
    pub cache: Arc<Mutex<LruCache<String, i64>>>,       
}

impl Actor for InboundCacheProcessor<'static> {
    type Context = SyncContext<Self>;
}

impl<'a> Handler<SearchResponse> for InboundCacheProcessor<'static> {
    type Result = Result<i64>;

    #[cfg_attr(feature="flame_it", flame)]
    fn handle(&mut self, msg: SearchResponse, _: &mut Self::Context) -> Self::Result {
        let cache = {self.cache.clone()};
        {
            match cache.lock() {
                Ok(mut guard) => if let Some(v) = guard.get(&msg.0) {
                    debug!("Cache hit");
                    return Ok((*v).to_owned());
                }                    
                Err(poisoned) => {
                    return Err(Error::from_kind(
                                ErrorKind::Poisoned(
                                    String::from(format!("{}", poisoned))
                                ))
                           ); 
                }
            }                
        }

        // info!("Got: \n{}", String::from_utf8(msg.1.clone())?);

        let res = self.pikkr.parse(&msg.1)?;
            if res.len() == 2 {
                if let Some(count) = res[0] {
                    let c_str = ::std::str::from_utf8(count)?;
                    let c = c_str.parse::<i64>()?;
                    let qtd = { 
                        if c == 1 {
                            if let Some(qtdvec) = res[1] {
                                let qtd_str = ::std::str::from_utf8(qtdvec)?;
                                if qtd_str.len() > 3 {
                                    let q : Vec<&str> = qtd_str.split(',').collect();
                                    q.len() as i64
                                } else {
                                    0
                                }
                            } else {
                                return Err(Error::from_kind(ErrorKind::Msg(String::from(
                                    "corrupt data!"))));
                            }
                        } else {
                            0
                        }
                    };

                    match cache.lock() {
                        Ok(mut guard) => {
                            trace!("Cached: \t{}", msg.0);                            
                            let _ = guard.insert(msg.0, qtd);
                            trace!("cache len: {}", guard.len());
                            return Ok(qtd);
                        }  
                        Err(poisoned) => {
                            return Err(Error::from_kind(
                                                        ErrorKind::Poisoned(
                                                            String::from(format!("{}", poisoned))
                                                        )
                                                    )
                            ); 
                        }                         
                    }
                } else {
                    return Err(Error::from_kind(
                        ErrorKind::Msg(String::from("unexpected data to parse"))
                    ));                    
                }
            } else {
                return Err(Error::from_kind(
                    ErrorKind::Msg(String::from("unexpected data to parse"))
                ));
            }
    }
}

pub struct ReadExecutor {
    pub store: Store,
    pub films: Addr<Syn, FilmsExecutor>,
}

impl Actor for ReadExecutor {
    type Context = SyncContext<Self>;
}

impl Handler<ReadPlanets> for ReadExecutor {
    type Result = Result<Value>;

    #[cfg_attr(feature="flame_it", flame)]
    fn handle(&mut self, _: ReadPlanets, _: &mut Self::Context) -> Self::Result {
        let ref mut store = self.store;
        let films = {self.films.clone()};
        let mut res1 : Vec<InnerPlanet> = 
            QueryBuilder::new(store, r#"[:find ?u, ?n, ?c, ?t  
                                         :where [?x :planet/uuid ?u]
                                                [?x :planet/name ?n]
                                                [?x :planet/climate ?c]
                                                [?x :planet/terrain ?t]
                                        ]"#)
                .execute_rel()?
                .into_iter()
                .map(|row| {
                    debug!("retrieving database");
                    let uuid = row.get(0).map_or(String::from(""), |t| t.to_owned().into_uuid_string().expect("uuid"));
                    let name = row.get(1).map_or(Arc::new(String::from("")), |t| t.to_owned().into_string().expect("name"));
                    let climate = row.get(2).map_or(Arc::new(String::from("")), |t| t.to_owned().into_string().expect("climate"));
                    let terrain = row.get(3).map_or(Arc::new(String::from("")), |t| t.to_owned().into_string().expect("terrain"));
                    let n = name.as_ref().clone();
                    trace!("sending ReadFilms req");
                    let req = films.send(ReadFilms(n)); // will parallel send before waiting
                    trace!("sent ReadFilms req");                    
                    InnerPlanet{uuid: uuid, name: name, climate: climate, terrain: terrain, request: Some(req)}
                })
                .collect();
        let res2 : Vec<Value> = res1.iter_mut()
                .map(|t| {
                    let (tx, rx) = bounded::<i64>(1);
                    debug!("waiting for future in ReadExecutor");
                    // let r = ::std::mem::replace(&mut t.request, None);
                    let _ = t.request.take().unwrap().wait()
                        .map_err(|e| {
                            warn!("got error 5: \t {}", e);
                        })
                        .and_then(|r| {
                            debug!("got r on ReadExecutor: \t {:?}", r);
                            let qtd = match r {
                                Ok(rx) => match rx.recv() {
                                    Ok(rst) => match rst {
                                        Ok(v) => v,
                                        Err(e) => {
                                            warn!("got error: \t {}", e);
                                            -1i64
                                        },
                                    }
                                Err(e) => {
                                    warn!("got error: \t {}", e);
                                    -1i64
                                }},
                                Err(e) => {
                                    warn!("got error: \t {}", e);
                                    -1i64
                                }
                            };
                            match tx.send(qtd) {
                                Ok(_) => trace!("qtd sent on ReadExecutor"),
                                Err(e) => warn!("got error 6: \t {}", e),
                            }
                            Ok(())
                        });
                    debug!("waiting for rx in ReadExecutor");
                    let qtd = match rx.recv() {
                        Ok(qtd) => qtd,
                        Err(e) => {
                            warn!("got error 7: \t {}", e);                            
                            -1
                        }
                    };

                    json!({"uuid": t.uuid,
                           "name": t.name,
                           "climate": t.climate,
                           "terrain": t.terrain,
                           "films": qtd
                          })
                    
                })
                .collect();

        Ok(json!(res2))
    }
}

impl Handler<SearchPlanet> for ReadExecutor {
    type Result = Result<Value>;

    #[cfg_attr(feature="flame_it", flame)]
    fn handle(&mut self, search : SearchPlanet, _: &mut Self::Context) -> Self::Result {
        let ref mut store = self.store;
        let films = {self.films.clone()};
        let s = format!("*{}*", search.0);
        let mut res1 : Vec<InnerPlanet> = 
            QueryBuilder::new(store, "[:find ?id ?n, ?c, ?t
                                  :in ?search
                                  :where [(fulltext $ :planet/name ?search) [[?x ?n _ _]]]
                                         [?x :planet/uuid ?id]
                                         [?x :planet/climate ?c]
                                         [?x :planet/terrain ?t]                                  
                        ]")
                .bind_value("?search", s)
                .execute_rel()?
                .into_iter()
                .map(|row| {
                    debug!("retrieving database");
                    let uuid = row.get(0).map_or(String::from(""), |t| t.to_owned().into_uuid_string().expect("uuid"));
                    let name = row.get(1).map_or(Arc::new(String::from("")), |t| t.to_owned().into_string().expect("name"));
                    let climate = row.get(2).map_or(Arc::new(String::from("")), |t| t.to_owned().into_string().expect("climate"));
                    let terrain = row.get(3).map_or(Arc::new(String::from("")), |t| t.to_owned().into_string().expect("terrain"));
                    let n = name.as_ref().clone();
                    trace!("sending ReadFilms req");
                    let req = films.send(ReadFilms(n)); // will parallel send before waiting
                    trace!("sent ReadFilms req");                    
                    InnerPlanet{uuid: uuid, name: name, climate: climate, terrain: terrain, request: Some(req)}
                })
                .collect();
        let res2 : Vec<Value> = res1.iter_mut()
                .map(|t| {
                    let (tx, rx) = bounded::<i64>(1);
                    debug!("waiting for future in ReadExecutor");
                    // let r = ::std::mem::replace(&mut t.request, None);
                    let _ = t.request.take().unwrap().wait()
                        .map_err(|e| {
                            warn!("got error 5: \t {}", e);
                        })
                        .and_then(|r| {
                            debug!("got r on ReadExecutor: \t {:?}", r);
                            let qtd = match r {
                                Ok(rx) => match rx.recv() {
                                    Ok(rst) => match rst {
                                        Ok(v) => v,
                                        Err(e) => {
                                            warn!("got error: \t {}", e);
                                            -1i64
                                        },
                                    }
                                Err(e) => {
                                    warn!("got error: \t {}", e);
                                    -1i64
                                }},
                                Err(e) => {
                                    warn!("got error: \t {}", e);
                                    -1i64
                                }
                            };
                            match tx.send(qtd) {
                                Ok(_) => trace!("qtd sent on ReadExecutor"),
                                Err(e) => warn!("got error 6: \t {}", e),
                            }
                            Ok(())
                        });
                    debug!("waiting for rx in ReadExecutor");
                    let qtd = match rx.recv() {
                        Ok(qtd) => qtd,
                        Err(e) => {
                            warn!("got error 7: \t {}", e);                            
                            0
                        }
                    };

                    json!({"uuid": t.uuid,
                           "name": t.name,
                           "climate": t.climate,
                           "terrain": t.terrain,
                           "films": qtd
                          })
                    
                })
                .collect();

        Ok(json!(res2))
    }
}

impl Handler<GetPlanet> for ReadExecutor {
    type Result = Result<Value>;

    #[cfg_attr(feature="flame_it", flame)]
    fn handle(&mut self, id : GetPlanet, _: &mut Self::Context) -> Self::Result {    
        let ref mut store = self.store;
        let films = {self.films.clone()};

        let results = QueryBuilder::new(store, "[:find [?n, ?c, ?t]
                                  :in ?id
                                  :where [?x :planet/uuid ?id]
                                         [?x :planet/name ?n]
                                         [?x :planet/climate ?c]
                                         [?x :planet/terrain ?t]                                  
                        ]")
                .bind_value("?id", id.0)
                .execute_tuple()?;

        let out = { 
            if let Some(rec) = results {
                let (tx, rx) = bounded::<i64>(1);
                let n = rec[0].clone().into_string().expect("data avaliable").deref().clone();
                let _ = films.send(ReadFilms(n.clone())).wait()
                        .map_err(|e| {
                            warn!("got error 5: \t {}", e);
                        })
                        .and_then(|r| {
                            debug!("got r on ReadExecutor: \t {:?}", r);
                            let qtd = match r {
                                Ok(rx) => match rx.recv() {
                                    Ok(rst) => match rst {
                                        Ok(v) => v,
                                        Err(e) => {
                                            warn!("got error: \t {}", e);
                                            -1i64
                                        },
                                    }
                                Err(e) => {
                                    warn!("got error: \t {}", e);
                                    -1i64
                                }},
                                Err(e) => {
                                    warn!("got error: \t {}", e);
                                    -1i64
                                }
                            };
                            match tx.send(qtd) {
                                Ok(_) => trace!("qtd sent on ReadExecutor"),
                                Err(e) => warn!("got error 6: \t {}", e),
                            }
                            Ok(())
                        });
                debug!("waiting for rx in ReadExecutor");
                let qtd = match rx.recv() {
                    Ok(qtd) => qtd,
                    Err(e) => {
                        warn!("got error 7: \t {}", e);                            
                        -1
                    }
                };

                    json!({"uuid": id.0,
                           "name": n,
                           "climate": rec[1].clone().into_string(),
                           "terrain": rec[2].clone().into_string(),
                           "films": qtd
                          })
                
            } else {
                Value::Null
            }

        };
        Ok(json!(out))
    }
}