use errors::Result;
use clap;
use actix::System;
use log::LevelFilter;
use env_logger::Builder;
use std::env;
use handlers;
use pikkr::Pikkr;
use actors::{WriterExecutor,State, InboundCacheProcessor, ReadExecutor, FilmsExecutor};
use actix::{SyncArbiter,Actor, Syn, Addr};
use actix_web::{middleware, http, server, App, http::header, middleware::cors::Cors};
use lru_time_cache::LruCache;
use std::sync::{Mutex, Arc};
use std::path::Path;
use dirs::Directories;
use mentat::Store;

fn init_logger(pattern: &str) {
    // Always print backtrace on panic.
    env::set_var("RUST_BACKTRACE", "full");
	let mut builder = Builder::new();
	// Disable info logging by default for some modules:
	builder.filter(Some("hyper"), LevelFilter::Warn);
    builder.filter(Some("mio::timer"), LevelFilter::Warn);
	builder.filter(Some("mio::poll"), LevelFilter::Warn);            
	builder.filter(Some("tokio_reactor"), LevelFilter::Warn);
	builder.filter(Some("tokio_core"), LevelFilter::Warn);
	builder.filter(Some("tokio_threadpool"), LevelFilter::Warn);
	builder.filter(Some("actix_web::middleware::logger"), LevelFilter::Info);
	builder.filter(Some("actix_web::server::h1decoder"), LevelFilter::Warn);    
	builder.filter(Some("trust_dns_proto"), LevelFilter::Warn);
          
	// Enable info for others.
	builder.filter(None, LevelFilter::Info);

	if let Ok(lvl) = env::var("RUST_LOG") {
		builder.parse(&lvl);
	}

	builder.parse(pattern);
	builder.init();
}

fn get_api_port() -> u16 {
    let port_str = env::var("PORT").unwrap_or(String::new());
    port_str.parse().unwrap_or(8080)
}

fn interface(interface: &str) -> String {
    match interface {
        "all" => "0.0.0.0",
        "local" => "127.0.0.1",
        x => x,
    }
    .into()
}

pub fn run<I, T>(args: I) -> Result<()> where
	I: IntoIterator<Item = T>,
	T: Into<::std::ffi::OsString> + Clone,
{
	let yaml = load_yaml!("./cli.yml");
	let matches = clap::App::from_yaml(yaml).version(crate_version!()).get_matches_from_safe(args)?;
	let log_pattern = matches.value_of("log").unwrap_or("");
	init_logger(log_pattern);

    let port = value_t!(matches, "port", u16).unwrap_or(get_api_port());
    let inet = interface(matches.value_of("interface").unwrap());
    let db = matches.value_of("db");
    let expire = value_t!(matches, "expire", u64).unwrap();    

    let mut dirs = Directories::default();
    if let Some(dbpath) = db {
        dirs.db = String::from(dbpath);
    }
    let _ = dirs.create_dirs(); 

    let sys = System::new("swapi");    

    if !Path::new(&dirs.db).exists() {   
        let mut store = Store::open(&dirs.db).expect("open must not fail!");
        let t = "[
                  {:db/ident :planet/uuid    
                   :db/valueType :db.type/uuid  
                   :db/index true   
                   :db/unique :db.unique/identity                   
                   :db/cardinality :db.cardinality/one}            
                  {:db/ident :planet/name
                   :db/valueType :db.type/string
                   :db/index true 
                   :db/unique :db.unique/value   
                   :db/fulltext true                
                   :db/cardinality :db.cardinality/one}
                  {:db/ident :planet/climate
                   :db/valueType :db.type/string
                   :db/cardinality :db.cardinality/one}
                  {:db/ident :planet/terrain
                   :db/valueType :db.type/string
                   :db/cardinality :db.cardinality/one}                                   
                 ]";

                 
        let _  = store.transact(t).unwrap();
    }

    let time_to_live = ::std::time::Duration::from_secs(expire); // default 7 days
    let lru_cache = Arc::new(Mutex::new(LruCache::<String, i64>::with_expiry_duration(time_to_live)));
    let ccache = lru_cache.clone();
    
    let d = dirs.db.clone();
    let dr = dirs.db.clone();
    let queries = vec![
            "$.count".as_bytes(),
            "$.results.films".as_bytes(),
    ];         

    // Start db executor actors 
    let db_addr = SyncArbiter::start(1, move || {
        let store = Store::open(&d).expect("open store must not fail!");       
        WriterExecutor{store}
    });   

    let ccache3 = ccache.clone();
    let proc_addr = SyncArbiter::start(8, move || {
        let pikkr = Pikkr::new(&queries.clone(), 1).unwrap();         
        InboundCacheProcessor{pikkr: pikkr, cache: ccache3.clone()}
    });   

    let ccache2 = ccache.clone();
    let proc_addr2 = proc_addr.clone();
    let film_addr : Addr<Syn, _> = FilmsExecutor::create(move |ctx| {  
        ctx.set_mailbox_capacity(1000);
        FilmsExecutor{processor: Some(proc_addr2.clone()), cache: Some(ccache2.clone())}
    });          

    let read_addr = SyncArbiter::start(8, move || {
        let store = Store::open(&dr).expect("open store must not fail!");       
        ReadExecutor{store: store, films: film_addr.clone()}
    });      

    server::new(move || {            
        App::with_state(State{db: db_addr.clone(), processor: proc_addr.clone(), read: read_addr.clone(), cache: lru_cache.clone()})
            // enable logger
            .middleware(middleware::Logger::default())
            .configure(|app| Cors::for_app(app)
                .allowed_methods(vec!["GET", "POST", "DELETE"])
                .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                .allowed_header(header::CONTENT_TYPE)
                .max_age(3600)            
                .resource("/sw", |r| {
                    r.method(http::Method::POST).f(handlers::create);
                    r.method(http::Method::GET).f(handlers::read);
                }) 
                .resource("/sw/{uuid}", |r| { 
                    r.method(http::Method::DELETE).f(handlers::delete);
                    r.method(http::Method::GET).f(handlers::id);
                })
                .resource("/sw/", |r| r.method(http::Method::GET).with2(handlers::search))
                .register())
        })
        .bind((inet.as_str(), port)).unwrap()
        .shutdown_timeout(1)
        .start();        

    let _ = sys.run();

	Ok(())    
}