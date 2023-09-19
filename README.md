# rust_study
study proj.

<!--
 Copyright 2023 lzd
 
 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 
     http://www.apache.org/licenses/LICENSE-2.0
 
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
-->

# 0. 来源
https://github.com/tyrchen/geektime-rust/blob/master

# 1. scrape_url
抓取url，并下载为md格式

```
cd scrape_url && cargo build --quiet && target/debug/scrape_url
```

# 2. httpie
cli工具
给url发送get或post请求

```
cd httpie && cargo build --quiet && target/debug/httpie post https://httpbin.org/post a=1 b=2
```

# 3. tumbor
后端server
图片转换服务器：调整大小、剪切、加水印等

```

```