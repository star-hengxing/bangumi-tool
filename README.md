# bangumi-tool

通过 [Bangumi](https://bgm.tv/) API 导出个人收藏数据的命令行工具，支持 CSV 和 JSON 格式。

## 功能

- 获取全部收藏记录并在终端按状态分组展示。
- 导出为 CSV（带 UTF-8 BOM，兼容 Excel）或 JSON 格式。
- JSON 格式针对 LLM 优化：短键名、省略空字段、紧凑输出。
- 状态标签根据条目类型自动适配（看过/玩过/读过/听过等）。
- 可选 `--detail` 模式获取每个条目的章节列表和观看进度。
- 基于文件的缓存（`.bgm_cache/`），支持断点续传。
- 内置请求限速（每次请求间隔 5 秒）。

## 使用

### 1. 配置访问令牌

从 [Bangumi 开发者设置](https://next.bgm.tv/demo/access-token) 获取令牌。

设置环境变量：

```bash
export BANGUMI_ACCESS_TOKEN=your_token_here
```

或在工作目录下创建 `.bgm_token` 文件：

```bash
echo "your_token_here" > .bgm_token
```

### 2. 运行

```bash
# 默认：获取收藏，终端展示摘要，导出 CSV 和 JSON
bangumi-tool

# 仅导出 JSON
bangumi-tool -f json

# 导出 CSV 到指定目录
bangumi-tool -f csv -o ./exports

# 获取每个条目的章节和进度详情
bangumi-tool --detail

# 忽略缓存，重新获取
bangumi-tool --no-cache

# 启用调试日志
bangumi-tool --debug
```

### 命令行选项

```
bangumi-tool [OPTIONS]

Options:
  -f, --format <FORMAT>  导出格式: json, csv, all [默认: all]
  -o, --output <DIR>     输出目录 [默认: .]
      --detail           获取每个条目的章节和进度详情
      --no-cache         忽略缓存，重新获取所有数据
      --debug            启用调试日志（输出 HTTP 请求和响应）
  -h, --help             打印帮助信息
```

## 输出

### 终端摘要（默认模式）

收藏按状态分组展示，标签根据条目类型自动适配：

```
== 在看/在玩/在读/在听 (15) ==
  --- 在看 (10) ---
    某动画 [动画]
  --- 在玩 (5) ---
    某游戏 [游戏] [8分]

== 看过/玩过/读过/听过 (120) ==
  --- 看过 (80) ---
    ...
  --- 玩过 (30) ---
    ...
  --- 读过 (10) ---
    ...
```

各条目类型对应的状态标签：

| 条目类型      | 想   | 在   | 过   | 搁置 | 抛弃 |
| ------------- | ---- | ---- | ---- | ---- | ---- |
| 动画 / 三次元 | 想看 | 在看 | 看过 | 搁置 | 抛弃 |
| 游戏          | 想玩 | 在玩 | 玩过 | 搁置 | 抛弃 |
| 书籍          | 想读 | 在读 | 读过 | 搁置 | 抛弃 |
| 音乐          | 想听 | 在听 | 听过 | 搁置 | 抛弃 |

### CSV

CSV 保留完整字段，适合 Excel 查看。

默认模式列：名称，名称 (中文)，条目类型，地址，状态，最后标注，我的评分，我的标签，我的评论

`--detail` 模式额外列：完成度，完成度 (百分比)，完成单集

### JSON

JSON 针对 LLM 读取优化，节省 token：

- 优先使用中文名，原名不同时才附加 `name_orig`。
- 短键名：`type`、`status`、`updated`、`rating`。
- 空字段（评分、标签、评论）省略不输出。
- `rating` 为数字类型而非字符串。
- 紧凑格式，无缩进换行。

示例：

```json
[
  {
    "name": "命运石之门",
    "name_orig": "Steins;Gate",
    "type": "动画",
    "status": "看过",
    "updated": "2025-01-01 12:00:00",
    "rating": 10
  },
  { "name": "塞尔达传说", "type": "游戏", "status": "在玩", "updated": "2025-06-15 18:30:00" }
]
```

`--detail` 模式额外字段：`progress`（如 `"12/24"`）、`progress_pct`（如 `"50%"`）、`watched`（如 `"1-5,7,9-12"`）。

## 缓存与断点续传

API 响应缓存在 `.bgm_cache/` 目录，再次运行时自动复用缓存，使用 `--no-cache` 清除缓存并重新获取。

## 从源码构建

```bash
cargo build --release
```

# Credits

1. [bangumi-takeout-py](https://github.com/bangumi-takeout-py)
2. claude code & claude-opus-4-6
