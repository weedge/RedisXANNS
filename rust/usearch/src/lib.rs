#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::env;
use std::sync::RwLock;

//#[allow(dead_code, unused_variables, unused_mut)]
mod types;
use types::{IndexOpts, IndexRedis, USEARCH_INDEX_REDIS_TYPE};

use redis_module::{redis_module, Context, NextArg, RedisError, RedisResult, RedisString, Status};
use usearch::Index;

static PREFIX: &str = "usearch";
static MODULE_NAME: &str = "redisxann-usearch";
static PATH_DIR_NAME: &str = "serialization_file_path_dir";

lazy_static! {
    static ref INDICES: RwLock<HashMap<String, Index>> = RwLock::new(HashMap::new());

    // just use init load args, then read it's args for cmd,,
    static ref MODULE_ARGS_MAP: RwLock<HashMap<String, String>> = {
        let mut m = HashMap::new();
        m.insert(PATH_DIR_NAME.to_string(), env::current_dir().unwrap().to_string_lossy().to_string());
        RwLock::new(m)
    };
}

// create_index
// cmd: usearch.index.create indexName [algo_param_key algo_param_value]
// cmd eg: usearch.index.create idx0 dim 3 m 10 efcon 12 metric ip quantization f32
// return "OK" or error
fn create_index(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    ctx.auto_memory();
    if args.len() < 12 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let name = format!("{}.{}", PREFIX, args.next_str()?);

    let index_name = ctx.create_string(name.clone());
    if args.next_str()?.to_lowercase() != "dim" {
        return Err(RedisError::WrongArity);
    }
    let dim = args.next_u64()? as usize;

    if args.next_str()?.to_lowercase() != "m" {
        return Err(RedisError::WrongArity);
    }
    let m = args.next_u64()? as usize;

    if args.next_str()?.to_lowercase() != "efcon" {
        return Err(RedisError::WrongArity);
    }
    let ef_construction = args.next_u64()? as usize;

    if args.next_str()?.to_lowercase() != "metric" {
        return Err(RedisError::WrongArity);
    }
    let metric = args.next_str()?.to_lowercase();

    if args.next_str()?.to_lowercase() != "quantization" {
        return Err(RedisError::WrongArity);
    }
    let quantization = args.next_str()?.to_lowercase();

    // get redisType value
    let key = ctx.open_key_writable(&index_name);
    match key.get_value::<IndexRedis>(&USEARCH_INDEX_REDIS_TYPE)? {
        Some(_) => {
            return Err(RedisError::String(format!(
                "Index: {} already exists",
                &index_name
            )));
        }
        None => {
            let mut opts = IndexOpts::default();
            opts.dimensions = dim;
            opts.connectivity = m;
            opts.expansion_add = ef_construction;
            opts.metric = metric.into();
            opts.quantization = quantization.into();

            // create index
            let mut redis_idx = IndexRedis::default();
            let idx = usearch::Index::new(&opts.into()).unwrap();
            redis_idx.index_capacity = idx.capacity();
            redis_idx.index_size = idx.size();
            redis_idx.serialized_length = idx.serialized_length();
            redis_idx.serialization_file_path = MODULE_ARGS_MAP
                .read()
                .unwrap()
                .get(PATH_DIR_NAME)
                .unwrap()
                .to_string();
            redis_idx
                .serialization_file_path
                .push_str(format!("/{}", name).as_str());
            redis_idx.index = Some(idx);

            // set redisType value
            ctx.log_debug(format!("create Usearch Index {:?}", redis_idx).as_str());
            key.set_value::<IndexRedis>(&USEARCH_INDEX_REDIS_TYPE, redis_idx.into())?;
        }
    }

    Ok("OK".into())
}

fn get_index(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    ctx.auto_memory();
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    ctx.log_notice(format!("{:?}", args).as_str());
    Ok("".into())
}

fn del_index(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    ctx.auto_memory();
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    ctx.log_notice(format!("{:?}", args).as_str());
    Ok("".into())
}

fn scan_index(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    ctx.auto_memory();
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    ctx.log_notice(format!("{:?}", args).as_str());
    Ok("".into())
}

#[cfg(not(test))]
macro_rules! get_allocator {
    () => {
        redis_module::alloc::RedisAlloc
    };
}

#[cfg(test)]
macro_rules! get_allocator {
    () => {
        std::alloc::System
    };
}

redis_module! {
    name: MODULE_NAME,
    version: 1,
    allocator: (get_allocator!(), get_allocator!()),
    data_types: [USEARCH_INDEX_REDIS_TYPE],
    init: init,
    commands: [
        [format!("{}.index.create", PREFIX), create_index, "write", 0, 0, 0],
        [format!("{}.index.get", PREFIX), get_index, "readonly", 0, 0, 0],
        [format!("{}.index.del", PREFIX), del_index, "write", 0, 0, 0],
        [format!("{}.index.scan", PREFIX), scan_index, "readonly", 0, 0, 0],
    ],
}

fn init(ctx: &Context, args: &[RedisString]) -> Status {
    if args.len() % 2 != 0 {
        ctx.log_warning(
            format!(
                "module arguments len {}, must be key:value pairs",
                args.len()
            )
            .as_str(),
        );
        return Status::Err;
    }

    for i in (0..args.len()).step_by(2) {
        MODULE_ARGS_MAP.write().unwrap().insert(
            args[i].to_string_lossy().to_string(),
            args[i + 1].to_string_lossy().to_string(),
        );
    }
    ctx.log_debug(format!("{:?}", MODULE_ARGS_MAP.read().unwrap()).as_str());

    Status::Ok
}
