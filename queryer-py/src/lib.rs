// 将queryer项目作为lib，通过pyo3给queryer-py项目使用: pip install marturin && maturin develop --bindings pyo3  # 使用 pyo3 绑定类型
// Test: 
// import queryer_py
// sql = queryer_py.example_sql()
// print(sql)
// print(queryer_py.query(sql, 'csv'))
// Output:
// SELECT location name, total_cases, new_cases, total_deaths, new_deaths FROM https://raw.githubusercontent.com/owid/covid-19-data/master/public/data/latest/owid-covid-latest.csv WHERE new_deaths > 10 ORDER BY new_cases DESC LIMIT 2
// name,total_cases,new_cases,total_deaths,new_deaths
// Romania,3429120.0,7171.0,68296.0,14.0
// Italy,25977012.0,4122.0,191370.0,21.0


use pyo3::{exceptions, prelude::*};

#[pyfunction]
pub fn example_sql() -> PyResult<String> {
    Ok(queryer::example_sql())
}

#[pyfunction]
pub fn query(sql: &str, output: Option<&str>) -> PyResult<String> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let data = rt.block_on(async { queryer::query(sql).await.unwrap() });
    match output {
        Some("csv") | None => Ok(data.to_csv().unwrap()),
        Some(v) => Err(exceptions::PyTypeError::new_err(format!(
            "Output type {} not supported",
            v
        ))),
    }
}

#[pymodule]
fn queryer_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(query, m)?)?;
    m.add_function(wrap_pyfunction!(example_sql, m)?)?;
    Ok(())
}