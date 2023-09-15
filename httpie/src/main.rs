
// 主入口

// 用///: clap会自动生成帮助文档

// 1. 写parser: cargo build --quiet && target/debug/httpie post httpbin.org/post a=1 b=2
// Output: Opts { subcmd: Post(Post { url: "httpbin.org/post", body: ["a=1", "b=2"] }) }
use clap::Parser;

// 2. 通过额外验证函数和trait来做字符串检查
// Test1: cargo build --quiet && target/debug/httpie post httpbin.org/post a=1 b=2
// OutPut1: error: Invalid value "httpbin.org/post" for '<URL>': relative URL without a base
// Test2: cargo build --quiet && target/debug/httpie post http://httpbin.org/post a=1 b=2
// OutPut2: Opts { subcmd: Post(Post { url: "http://httpbin.org/post", body: [KvPair { k: "a", v: "1" }, KvPair { k: "b", v: "2" }] }) }
use std::{str::FromStr};
use anyhow::{anyhow, Result};

// 3. 增加httpie核心功能
// Test1: cargo build --quiet && target/debug/httpie post https://httpbin.org/post a=1 b=2
// Output1: "{\n  \"args\": {}, \n  \"data\": \"{\\\"b\\\":\\\"2\\\",\\\"a\\\":\\\"1\\\"}\", \n  \"files\": {}, \n  \"form\": {}, \n  \"headers\": {\n    \"Accept\": \"*/*\", \n    \"Content-Length\": \"17\", \n    \"Content-Type\": \"application/json\", \n    \"Host\": \"httpbin.org\", \n    \"X-Amzn-Trace-Id\": \"Root=1-650428e4-579a76350b4575b23df266e2\"\n  }, \n  \"json\": {\n    \"a\": \"1\", \n    \"b\": \"2\"\n  }, \n  \"origin\": \"129.126.148.247\", \n  \"url\": \"https://httpbin.org/post\"\n}\n"
// Test2: cargo test
use reqwest::{header, Client, Response, Url};
use std::{collections::HashMap};
use mime::Mime;
use colored::*;


/// A naive httpie implementation in Rust
#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "biubiubiu")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand
}

// 子命令对应http方法
// clap中提供的宏，用于生成子命令
#[derive(Parser, Debug)]
enum SubCommand {
    Get(Get),
    Post(Post),
}

// Get子命令
/// httpie get
#[derive(Parser, Debug)]
struct Get {
    /// Http URL
    #[clap(parse(try_from_str = parse_url))]
    url: String,
}

async fn get(client: Client, args: &Get) -> Result<()> {
    let resp = client.get(&args.url).send().await?;
    println!("{:?}", resp.text().await?);
    Ok(())
}  

// post子命令，传入URL和body（多个可选key=value）
/// httpie post
#[derive(Parser, Debug)]
struct Post {
    /// Http URL
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    /// Http body
    #[clap(parse(try_from_str= parse_kv_pair ))]
    body: Vec<KvPair>,
}

async fn post(client: Client, args: &Post) -> Result<()> {
    // 解析出的kv pair放到HashMap中 传给json打包
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.k, &pair.v);
    }
    let resp = client.post(&args.url).json(&body).send().await?;
    println!("{:?}", resp.text().await?);
    Ok(())
}

// 打印服务器版本号 + 状态码
fn print_status(resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

// 打印服务器返回的 HTTP header
fn print_headers(resp: &Response) {
    for (name, value) in resp.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }

    println!();
}

/// 打印服务器返回的 HTTP body
fn print_body(m: Option<Mime>, body: &str) {
    match m {
        // 对于 "application/json" 我们 pretty print
        Some(v) if v == mime::APPLICATION_JSON => {
            println!("{}", jsonxf::pretty_print(body).unwrap().cyan())
        }

        // 其它 mime type，我们就直接输出
        // 在match中使用_，表示剩余所有情况
        _ => println!("{}", body),
    }
}

/// 打印整个响应
async fn print_resp(resp: Response) -> Result<()> {
    print_status(&resp);
    print_headers(&resp);
    let mime = get_content_type(&resp);
    let body = resp.text().await?;
    print_body(mime, &body);
    Ok(())
}

/// 将服务器返回的 content-type 解析成 Mime 类型
fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())
}



#[derive(Debug, PartialEq)]
struct KvPair {
    k: String,
    v: String,
}

fn parse_url(s: &str) -> Result<String> {
    // check s is valid url
    let _url: Url = s.parse()?;
    Ok(s.into())
}


// 实现FromStr trait，用于parse key=value
// https://doc.rust-lang.org/std/str/trait.FromStr.html
impl FromStr for KvPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split =  s.split("=");
        let err = || anyhow!(format!("Failed to parse {}", s));
        // 迭代器返回Some(T)/None，转换成Ok(T)/Err(E), 然后用?处理错误
        Ok(Self {
            // 从split中取出两个值，如果没有则返回err
            k: (split.next().ok_or_else(err)?).to_string(),
            v: (split.next().ok_or_else(err)?).to_string(),
        })
    }
}

fn parse_kv_pair(s: &str) -> Result<KvPair> {
    Ok(s.parse()?)
}


// asyn 异步函数
// tokio 宏，自动添加处理异步的运行时
#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    // 生成一个HTTP client
    let client = Client::new();
    let result = match opts.subcmd {
        SubCommand::Get(ref args) => get(client, args).await?,
        SubCommand::Post(ref args) => post(client, args).await?,
    };

    Ok(result)
}

// Test: cargo test 时编译

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_works() {
        assert!(parse_url("abc").is_err());
        assert!(parse_url("http://abc.xyz").is_ok());
        assert!(parse_url("https://httpbin.org/post").is_ok());
    }

    #[test]
    fn parse_kv_pair_works() {
        assert!(parse_kv_pair("a").is_err());
        assert_eq!(
            parse_kv_pair("a=1").unwrap(),
            KvPair {
                k: "a".into(),
                v: "1".into()
            }
        );

        assert_eq!(
            parse_kv_pair("b=").unwrap(),
            KvPair {
                k: "b".into(),
                v: "".into()
            }
        );
    }
}