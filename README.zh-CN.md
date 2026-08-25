# Prose Lint ✨🚀

**简体中文** · [English](README.md)

> 接下来，我将用最直白、最不绕弯子的方式告诉你：在这个瞬息万变、日新月异、机遇与挑战并存的 AI 新时代，Prose Lint 不只是一个扫描器，更是一场关于文字生产力的深刻变革。它以稳健、无缝、全方位的方式赋能你的表达，深入探索技术写作的复杂肌理，精准洞察每一个至关重要的语言细节，在纷繁复杂的内容生态中为你点亮前行的灯塔。值得注意的是，无论你的句子走了多远，它都会稳稳地接住你。让我们携手并进，共同开启自然写作的新篇章。🌐💡

对，上面这段就是故意的：它是一份披着 README 外衣的测试样本。

Prose Lint 是一个快速、确定性的英文 prose linter，用来找出 LLM 和
Codex 常见的公式化表达。仓库同时提供一个 Agent Skill，指导 agent 在不改动
事实、证据和不确定性的前提下修订文本。

它提供的是编辑线索，不判断一段文字由人还是模型创作。

## 📋 1 键复制给 Agent 安装

点击下面代码块右上角的复制按钮，把整段发给 Codex、Claude Code、Hermes、
Cursor 或其他能执行命令的 coding agent：

```text
帮我安装并配置 Prose Lint：https://github.com/bkmashiro/prose-lint 。先完整阅读 https://github.com/bkmashiro/prose-lint/blob/main/AGENT_INSTALL.md，然后严格按指南执行：安装 CLI，使用你原生的 skill 机制安装 prose-lint skill，并完成文档里的 smoke test。已有 Rust 工具链就直接复用；缺少前置依赖时，通过当前环境正常且安全的方式安装。不要只 clone 仓库，不要只告诉我应该执行什么命令，也不要停在计划阶段。全部完成后，向我报告 CLI 的准确路径、skill 的准确路径或 ID、版本输出，以及 smoke test 的真实结果。除非当前 agent 原生采用项目级 skill，否则不要修改我当前的项目。
```

Agent 应阅读 [`AGENT_INSTALL.md`](AGENT_INSTALL.md)。其中包含环境探测、Hermes、
Codex、Claude Code 和通用 Agent Skills 的安装路径，以及验证、更新和卸载步骤。
手动安装请继续看[安装](#安装)。

## 为什么还要做一个 prose linter？

多数“AI 词库”有两个问题：把每一次命中都当成错误，以及把写作质量和作者身份
混为一谈。Prose Lint 将它们分开处理：

- 高置信度规则关注可能掩盖证据、夸大 claim 或降低技术精度的表达；
- 上下文规则识别 Codex 常用的工程隐喻和抽象词聚集，但不禁用合法术语；
- 407 个有实证来源的 style words 作为低置信度审阅线索；
- 来源研究的全部 900 条记录都保留在仓库中，scanner 不会启用其中的 content
  words。

低置信度词汇永远不会让 strict mode 失败。

## 性能设计

Scanner 不会让每条表达式分别遍历整篇文档：

1. 固定词和短语编译成一个不区分 ASCII 大小写的 Aho–Corasick 自动机；
2. regex 编译成 `RegexSet`，只对已经确定命中的表达式再次定位；
3. 上下文 cluster 在段落内聚合，不构造组合爆炸的 regex；
4. 每个文件只做一次 Markdown masking 和换行索引；
5. 目录使用 Rayon 跨文件并行，worker 共享编译后的 scanner。

Release binary 只有六个直接依赖：`aho-corasick`、`glob`、`regex`、`rayon`、
`serde` 和 `serde_json`。没有 parser framework、async runtime、network client
或 NLP model。

## 安装

```bash
git clone https://github.com/bkmashiro/prose-lint.git
cd prose-lint
cargo install --path .
```

也可以不保留源码目录：

```bash
cargo install --locked --git https://github.com/bkmashiro/prose-lint.git prose-lint
```

规则数据嵌入可执行文件，扫描时不需要网络或独立的数据目录。

## 使用

```bash
# 使用默认 technical profile 扫描单个文件
prose-lint scan README.md

# 扫描目录；自动跳过构建和依赖目录
prose-lint scan docs/

# 只扫描当前目录的 Typst 文件；引号确保由 CLI 而不是 shell 展开
prose-lint scan '*.typ'

# 递归扫描所有 Typst 文件
prose-lint scan '**/*.typ'

# 显示所有低置信度实证词汇命中
prose-lint scan paper.md --profile academic --all

# 输出 JSON，供编辑器或 agent 使用
prose-lint scan docs/ --format json

# 仅在出现 High finding 时返回失败
prose-lint scan docs/ --strict

# 限制文件级并行度
prose-lint scan docs/ --jobs 4
```

支持 `.md`、`.mdx`、`.txt`、`.rst`、`.adoc`、`.tex` 和 `.typ`。Fenced
code、inline code 和 URL 会被屏蔽，同时保留原始 byte offset。位置参数原生
支持 `*`、`?`、`[ab]` 一类字符组以及递归 `**` 通配符。建议用单引号包住
pattern，让 shell 把它原样交给 CLI；没有匹配项时会明确报错，不会静默完成
一次空扫描。

### 仓库专属附加词库

在仓库根目录添加 `.prose-lint.json`，即可扩展 literal 词库，不需要修改内置
数据集：

```json
{
  "extra_terms": [
    "magic surface",
    {
      "term": "lands cleanly",
      "severity": "high",
      "message": "请使用本仓库具体的合并术语。",
      "suggestion": "直接说明实际操作。"
    }
  ]
}
```

字符串条目默认是 `medium`。详细条目支持 `low`、`medium`、`high`，其中
`message` 和 `suggestion` 可省略。附加词按大小写不敏感、带词边界的 literal
匹配处理，不接受 regex；Markdown code 和 URL 仍然会被屏蔽。

扫描每个文件时，Prose Lint 会向上查找最近的 `.prose-lint.json`，到该文件
所属 Git 根目录为止。因此同一个命令可以同时扫描多个仓库，并分别采用各自的
词库。`--config PATH` 可以强制所有输入使用同一个配置。专属词是仓库明确指定
的规则，因此即使是 `low` 也始终显示；只有 `high` 会影响 `--strict`。

可用 profile：

```text
technical  academic  pr  commit  casual  marketing
```

## 置信度

### High

逐项审阅。当前高置信度类别包括：

- 空洞的重要性标记；
- 装饰性的对比句式；
- stock implication sentence；
- 修辞膨胀；
- 没有来源支持的设计理由；
- 可能擅自加强原 claim 的证据动词；
- meta-writing；
- `honest shape`、`clean boundary` 等 Codex 式模糊形容；
- chatbot residue。

### Medium

这些模式必须结合上下文判断：

- 形式化 signposting 和 participial tail；
- 抽象名词堆叠；
- 声称技术对象会 “live”、“own” 或 “carry” 某物；
- 声称改动会 “land”；
- `boundary / surface / contract / posture` 密集共现；
- 宣传式措辞和过高的 em dash 密度。

如果一个技术术语准确描述了概念，就应保留。

### Low

实证词汇来自 Kobak 等人的 excess vocabulary 研究，研究对象是 biomedical
abstract，因此不能假设它能直接迁移到所有领域。Prose Lint 默认隐藏单个 Low
命中，只显示总数；使用 `--all` 才会展开。

## Agent Skill

`skills/prose-lint/SKILL.md` 告诉 agent 如何运行 scanner、解释不同置信度、
保留 claim 和 uncertainty，并验证改写结果。

Skill 本身保持简短。完整目录由可执行文件加载，agent context 只需要接收当前
文档真正命中的 finding。

### Hermes 直接安装

```bash
hermes skills install https://raw.githubusercontent.com/bkmashiro/prose-lint/main/skills/prose-lint/SKILL.md
```

其他 agent 的准确安装位置见 [`AGENT_INSTALL.md`](AGENT_INSTALL.md)。

## 规则数据

- `data/rules.json`：curated phrase、regex、profile、提示和编辑动作；
- `data/excess-vocabulary.json`：900 条研究记录，其中 407 条作为 Low style
  signal 启用；
- `THIRD_PARTY_NOTICES.md`：数据来源、引用和许可证。

Curated rules 保存在数据文件中。增加普通短语通常不需要修改 Rust engine。

## 开发

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

生成可重复的本地 benchmark corpus：

```bash
python3 scripts/generate_benchmark_corpus.py /tmp/prose-lint-bench
/usr/bin/time -lp target/release/prose-lint scan /tmp/prose-lint-bench >/dev/null
```

最近一次本地 smoke benchmark 和环境记录在
[`BENCHMARKS.md`](BENCHMARKS.md)。

## 限制

- Scanner 当前面向英文 prose；中文 README 中的整活文案不会得到有意义的中文
  style 诊断；
- 它使用确定性的表层和结构规则，不做语义推理；
- 没有 finding 不代表文本自然、正确或由人创作；
- 出现 finding 也不意味着必须改写，最终仍由上下文和文档目的决定。

## 许可证

项目代码和 curated rules 使用 MIT License。转换后的研究数据保留上游 MIT
notice，详见 `THIRD_PARTY_NOTICES.md`。
