// 1. 写parser: cargo build --quiet && target/debug/httpie post httpbin.org/post a=1 b=2
// Output: Opts { subcmd: Post(Post { url: "httpbin.org/post", body: ["a=1", "b=2"] }) }
use clap::Parser;

// 2. 通过额外验证函数和trait来做字符串检查
// Test1: cargo build --quiet && target/debug/httpie post httpbin.org/post a=1 b=2
// OutPut1: error: Invalid value "httpbin.org/post" for '<URL>': relative URL without a base
// Test2: cargo build --quiet && target/debug/httpie post http://httpbin.org/post a=1 b=2
// OutPut2: Opts { subcmd: Post(Post { url: "http://httpbin.org/post", body: [KvPair { k: "a", v: "1" }, KvPair { k: "b", v: "2" }] }) }
use std::str::FromStr;
use anyhow::{anyhow, Result};
use reqwest::{header, Client, Response, Url};

// 主入口

// 用///: clap会自动生成帮助文档

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

#[derive(Debug)]
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


fn main() {
    let opts: Opts = Opts::parse();
    println!("{:?}", opts);
}
