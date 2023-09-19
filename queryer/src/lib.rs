// Test1: 通过example/dialect.rs 打印ast: cargo run --example dialect
// Output: Ok(
    // [
    //     Query(
    //         Query {
    //             with: None,
    //             body: Select(
    //                 Select {
    //                     distinct: false,
    //                     top: None,
    //                     projection: [
    //                         ExprWithAlias {
    //                             expr: Identifier(
    //                                 Ident {
    //                                     value: "a",
    //                                     quote_style: None,
    //                                 },

// Test2: 通过example/covid.rs 打印表格: cargo run --example covid
// Output: 输出最近死亡大于10的国家
//+-----------+-------------+-----------+--------------+------------+
// | location  | total_cases | new_cases | total_deaths | new_deaths |
// | ---       | ---         | ---       | ---          | ---        |
// | str       | f64         | f64       | f64          | f64        |
// +===========+=============+===========+==============+============+
// | "Finland" | 1.484646e6  | 315       | 1.0185e4     | 14         |
// +-----------+-------------+-----------+--------------+------------+
// | "Italy"   | 2.5977012e7 | 4122      | 1.9137e5     | 21         |
// +-----------+-------------+-----------+--------------+------------+
// | "Romania" | 3.42912e6   | 7171      | 6.8296e4     | 14         |
// +-----------+-------------+-----------+--------------+------------+

// 3. 完成convert.rs: 把 sqlparser 解析出来的 AST 转换成 polars 定义的 AST

// 4. 完成fetcher.rs,loader.rs : 从远程获取数据、加载数据功能，用 trait 抽取 fetch 的逻辑，定义好接口，然后改变 retrieve_data 的实现

// 5. 完成lib.rs：定义业务逻辑及业务逻辑要使用的DataSet

// 6. Test3: 通过example/covid.rs用库的方式引用queryer lib,打印sql语句结果表格: cargo run --example covid
// +-----------+-------------+-----------+--------------+------------+
// | location  | total_cases | new_cases | total_deaths | new_deaths |
// | ---       | ---         | ---       | ---          | ---        |
// | str       | f64         | f64       | f64          | f64        |
// +===========+=============+===========+==============+============+
// | "Romania" | 3.42912e6   | 7171      | 6.8296e4     | 14         |
// +-----------+-------------+-----------+--------------+------------+
// | "Italy"   | 2.5977012e7 | 4122      | 1.9137e5     | 21         |
// +-----------+-------------+-----------+--------------+------------+
// | "Finland" | 1.484646e6  | 315       | 1.0185e4     | 14         |
// +-----------+-------------+-----------+--------------+------------+

use anyhow::{anyhow, Result};
use polars::prelude::*;
use sqlparser::parser::Parser;
use std::convert::TryInto;
use std::ops::{Deref, DerefMut};
use tracing::info;

// 调用自己的其他包
mod convert;
mod dialect;
mod loader;
mod fetcher;
use convert::Sql;
use loader::detect_content;
use fetcher::retrieve_data;

// pub use 可以把其他包的内容暴露给外部(queryer-py)使用
pub use dialect::example_sql;
pub use dialect::TyrDialect;

#[derive(Debug)]
pub struct DataSet(DataFrame);

/// 让 DataSet 用起来和 DataFrame 一致
impl Deref for DataSet {
    type Target = DataFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// 让 DataSet 用起来和 DataFrame 一致
impl DerefMut for DataSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DataSet {
    /// 从 DataSet 转换成 csv
    pub fn to_csv(&self) -> Result<String> {
        let mut buf = Vec::new();
        let writer = CsvWriter::new(&mut buf);
        writer.finish(self)?;
        Ok(String::from_utf8(buf)?)
    }
}

/// 从 from 中获取数据，从 where 中过滤，最后选取需要返回的列
pub async fn query<T: AsRef<str>>(sql: T) -> Result<DataSet> {
    let ast = Parser::parse_sql(&TyrDialect::default(), sql.as_ref())?;

    if ast.len() != 1 {
        return Err(anyhow!("Only support single sql at the moment"));
    }

    let sql = &ast[0];

    // 整个 SQL AST 转换成我们定义的 Sql 结构的细节都埋藏在 try_into() 中
    // 我们只需关注数据结构的使用，怎么转换可以之后需要的时候才关注，这是
    // 关注点分离，是我们控制软件复杂度的法宝。
    let Sql {
        source,
        condition,
        selection,
        offset,
        limit,
        order_by,
    } = sql.try_into()?;

    info!("retrieving data from source: {}", source);

    // 从 source 读入一个 DataSet
    // detect_content，怎么 detect 不重要，重要的是它能根据内容返回 DataSet
    let ds = detect_content(retrieve_data(source).await?).load()?;

    let mut filtered = match condition {
        Some(expr) => ds.0.lazy().filter(expr),
        None => ds.0.lazy(),
    };

    filtered = order_by
        .into_iter()
        .fold(filtered, |acc, (col, desc)| acc.sort(&col, desc));

    if offset.is_some() || limit.is_some() {
        filtered = filtered.slice(offset.unwrap_or(0), limit.unwrap_or(usize::MAX));
    }

    Ok(DataSet(filtered.select(selection).collect()?))
}
