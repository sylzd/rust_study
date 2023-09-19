// 1. 生成并添加pb mod(abi.proto,mod.rs): 
// Test: cargo build && cargo test 
// Output: 看是否生成abi.rs
mod pb;
use pb::*;

// 2. 引入http服务器,通过axum引入: 
// Test: cargo run &&  ../httpie/target/debug/httpie get "http://localhost:3000/image/CgoKCAjYBBCgBiADCgY6BAgUEBQKBDICCAM/https%3A%2F%2Fimages%2Epexels%2Ecom%2Fphotos%2F2470905%2Fpexels%2Dphoto%2D2470905%2Ejpeg%3Fauto%3Dcompress%26cs%3Dtinysrgb%26dpr%3D2%26h%3D750%26w%3D1260"
// Output: spec: ImageSpec { specs: [ Spec { data: Some( Resize( Resize { width: 600, ...
use axum::{
    extract::{Extension, Path}, 
    handler::get, 
    http::{HeaderMap, StatusCode, HeaderValue},
    Router, AddExtensionLayer,
};
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::convert::TryInto;

// 参数使用 serde 做 Deserialize，axum 会自动识别并解析
#[derive(Deserialize)]
struct Params {
    spec: String,
    url: String,
}

// 3. 获取源图，并缓存：
// Test: 在浏览器中输入打印出的 test url: http://localhost:3000/image/CgoKCAj0AxCgBiADCgY6BAgUEBQKBDICCAM/https%3A%2F%2Fencrypted%2Dtbn0%2Egstatic%2Ecom%2Fimages%3Fq%3Dtbn%3AANd9GcTQMG9VPeSdaGocXfIjFa0PxGgtc8DznVkt1bje56E%26s
// Output: 1. 显示图片 2. 第一次从网络获取图片，后面从缓存获取
// Sep 18 17:52:19.632  INFO retrieve_image{url="https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTQMG9VPeSdaGocXfIjFa0PxGgtc8DznVkt1bje56E&s"}: tumbor: Retrieve url
// Sep 18 17:52:28.421  INFO retrieve_image{url="https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTQMG9VPeSdaGocXfIjFa0PxGgtc8DznVkt1bje56E&s"}: tumbor: Match cache 6062661540816763442
// Sep 18 17:52:34.298  INFO retrieve_image{url="https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTQMG9VPeSdaGocXfIjFa0PxGgtc8DznVkt1bje56E&s"}: tumbor: Match cache 6062661540816763442
use bytes::Bytes;
use lru::LruCache;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tracing::{info, instrument};
// 注意anyhow中的Result和系统自带的是不一样的，封装了一层
use anyhow::Result;

type Cache = Arc<Mutex<LruCache<u64, Bytes>>>;

// 4. 增加图片处理功能，并将其封装为engine，以后可以替换为其他图片处理库
// 步骤：1. 增加engine mode: 添加engine trait定义及实现  2. 引入mod，如下三行代码 3. 修改generate函数，使用engine处理图片 4.  RUST_LOG=info cargo run --quiet
// Test: 调整图片质量级别（1~100）后浏览器输入：http://localhost:3000/image/CgoKCAj0AxCgBiADCgY6BAgUEBQKBDICCAM/https%3A%2F%2Fimg2.jiemian.com%2F101%2Foriginal%2F20170426%2F149321790767763800_a640x364.jpg
// Output: 图片会被打上rust logo水印
// Output2: 图片压缩模糊化显示，尺寸会变小(Quality:100->10 size:  153758 -> 14356)
mod engine;
use engine::{Engine, Photon};
use image::ImageOutputFormat;


#[tokio::main]
async fn main() {
    // 初始化 tracing
    tracing_subscriber::fmt::init();
    let cache: Cache = Arc::new(Mutex::new(LruCache::new(1024)));
    // 构建路由
    let app = Router::new()
        // `GET /image` 会执行 generate 函数，并把 spec 和 url 传递过去
        .route("/image/:spec/:url", get(generate))
        .layer(
            ServiceBuilder::new()
                .layer(AddExtensionLayer::new(cache))
                .into_inner(),
        );

    // 运行 web 服务器
    let addr = "127.0.0.1:3000".parse().unwrap();

    print_test_url("https://img2.jiemian.com/101/original/20170426/149321790767763800_a640x364.jpg");
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn generate(
    Path(Params { spec, url }): Path<Params>,
    Extension(cache): Extension<Cache>,
) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
    let spec: ImageSpec = spec
        .as_str()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let url: &str = &percent_decode_str(&url).decode_utf8_lossy();
    let data = retrieve_image(url, cache).await.map_err(|_| StatusCode::BAD_REQUEST)?;

    // TODO: 此处增加图片处理逻辑
    // 使用 image engine 处理
    let mut engine: Photon = data
    .try_into()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    engine.apply(&spec.specs);

    let image = engine.generate(ImageOutputFormat::Jpeg(100));

    info!("Finished processing: image size {}", image.len());
    let mut headers = HeaderMap::new();

    headers.insert("content-type", HeaderValue::from_static("image/jpeg"));
    Ok((headers, image))
}

#[instrument(level = "info", skip(cache))]
async fn retrieve_image(url: &str, cache: Cache) -> Result<Bytes> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let key = hasher.finish();

    let g = &mut cache.lock().await;
    let data = match g.get(&key) {
        Some(v) => {
            info!("Match cache {}", key);
            v.to_owned()
        }
        None => {
            info!("Retrieve url");
            let resp = reqwest::get(url).await?;
            let data = resp.bytes().await?;
            g.put(key, data.clone());
            data
        }
    };

    Ok(data)
}

// 调试辅助函数
fn print_test_url(url: &str) {
    use std::borrow::Borrow;
    let spec1 = Spec::new_resize(500, 800, resize::SampleFilter::CatmullRom);
    let spec2 = Spec::new_watermark(20, 20);
    let spec3 = Spec::new_filter(filter::Filter::Marine);
    let image_spec = ImageSpec::new(vec![spec1, spec2, spec3]);
    let s: String = image_spec.borrow().into();
    let test_image = percent_encode(url.as_bytes(), NON_ALPHANUMERIC).to_string();
    println!("test url: http://localhost:3000/image/{}/{}", s, test_image);
}