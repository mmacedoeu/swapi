#![cfg_attr(feature="flame_it", feature(plugin, custom_attribute))]
#![cfg_attr(feature="flame_it", plugin(flamer))]

#[cfg(feature="flame_it")]
extern crate flame;
extern crate app_dirs;
extern crate env_logger;
extern crate actix;
extern crate actix_web;
extern crate futures;
extern crate bytes;
extern crate pikkr;
extern crate lru_time_cache;
extern crate uuid;
extern crate failure;
extern crate crossbeam_channel;

#[macro_use]
extern crate mentat;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate derive_error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

mod errors;
mod domain;
mod actors;
mod handlers;
mod dirs;
mod cli;

quick_main!(run);

fn run() -> errors::Result<()> {
	cli::run(::std::env::args())
}
