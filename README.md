# AGY 多账号配额中心与跨终端热重载系统 (agy-quota-center)

[![Release](https://img.shields.io/github/v/release/moxiuren/agy-quota-center?style=flat-square&color=blue)](https://github.com/moxiuren/agy-quota-center/releases)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows_x64-0078D6?style=flat-square&logo=windows)](https://microsoft.com)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

> **极速 Rust 原生构建 | 双周期配额智能熔断调度 | Win32 控制台协同热重载 | 专属 UUID 独立复原 | 沉睡周时钟预热 | 全局默认模型无缝切换**

---

## 目录 (Table of Contents)

- [一、项目背景与解决痛点](#一项目背景与解决痛点)
- [二、系统架构与核心特性 (Deep Dive)](#二系统架构与核心特性-deep-dive)
  - [1. 双周期智能调度评分算法 (Dual-Horizon Quota Scoring)](#1-双周期智能调度评分算法-dual-horizon-quota-scoring)
  - [2. Win32 控制台协同热重载与专属 UUID 独立复原 (Zero-Collision Resumption)](#2-win32-控制台协同热重载与专属-uuid-独立复原-zero-collision-resumption)
  - [3. 五阶梯自适应动态调频与消耗速率估算 (Adaptive Polling & Burn-Rate)](#3-五阶梯自适应动态调频与消耗速率估算-adaptive-polling--burn-rate)
  - [4. 沉睡周额度时钟预热激活器 (Weekly Clock Pre-warm Kickstart)](#4-沉睡周额度时钟预热激活器-weekly-clock-pre-warm-kickstart)
  - [5. 全局默认模型无缝切换系统 (Global Default Model Switcher)](#5-全局默认模型无缝切换系统-global-default-model-switcher)
  - [6. 后台脱机守护模式 (Headless Guard Daemon) & Windows Toast 气泡](#6-后台脱机守护模式-headless-guard-daemon--windows-toast-气泡)
  - [7. 合规性与系统级凭据安全 (Compliance & Credential Security)](#7-合规性与系统级凭据安全-compliance--credential-security)
- [三、交互式终端界面展示](#三交互式终端界面展示)
- [四、CLI 命令行指令速查 (Command Reference)](#四cli-命令行指令速查-command-reference)
- [五、源码编译与本地安装 (Build & Install)](#五源码编译与本地安装-build--install)
- [六、常见问题解答 (FAQ)](#六常见问题解答-faq)
- [七、免责声明 (Disclaimer)](#七免责声明-disclaimer)
- [八、开源许可 (License)](#八开源许可-license)

---

## 一、项目背景与解决痛点

在使用 Google Antigravity (`agy`) 官方 CLI 工具进行全天候沉浸式代码开发时，高频开发者通常会面临以下核心痛点：

1. **单账号配额瓶颈与开发中断**：单个 Google 账号的配额包含“5 小时爆发周期”与“7 天每周限额”。当重度编程遇到配额耗尽（HTTP 429）时，开发者被迫停下工作手动退出终端、重新鉴权登录，导致编码心流严重中断。
2. **多终端会话并发抢占死锁 (`Conversation already open`)**：
   - 官方恢复命令 `agy -c` 默认恢复最近的活跃会话。
   - 当开发者在多个独立终端窗口（如分别运行前端、后端、数据分析等子项目）并发执行切号重载时，所有窗口会因争抢同一个最新会话 ID 而触发死锁冲突。
3. **Claude 周额度时钟迟滞 (Dormant Clock Problem)**：
   - Claude 模型的 7 天周限额采用“首问激活机制”——若账号登录后未发起过有效交互，其 7 天刷新倒计时**根本不会启动**。
   - 备用账号即便闲置数周，首次使用时依然从第 7 天算起，无法享受到自然滚动的额度刷新红利。
4. **手动模型切换繁琐**：在 Claude 3.7 Sonnet、Gemini 2.5 Flash 与 Gemini 2.5 Pro 之间切换需要手动修改配置或每次启动传参，缺乏全局层面的快速统管机制。

**`agy-quota-center` (AGY 多账号配额中心)** 专为解决上述痛点而生。基于纯 Rust 编写，无运行时依赖，利用 Windows 底层系统调用与智能加权算法，提供全自动、无感知、零冲突的平滑开发体验。

---

## 二、系统架构与核心特性 (Deep Dive)

```text
+-----------------------------------------------------------------------------------+
|                           AGY Quota Center (v2.3.0)                               |
+-----------------------------------------------------------------------------------+
       |                                      |                               |
       v                                      v                               v
[双周期加权评分引擎]                  [Win32 协同热重载引擎]            [沉睡时钟预热激活器]
- 5h 爆发配额 (权重 0.7)              - WriteConsoleInput 键盘注入      - 极轻量探测 (Probe)
- 7d 周配额 (权重 0.3)                - 屏幕缓冲区正则 UUID 嗅探        - 提前启动 7 天倒计时
- 15% 临界智能安全熔断                - agy --conversation=<UUID>       - 账号池全面时钟激活
       |                                      |                               |
       +--------------------------------------+-------------------------------+
                                      |
                                      v
                        [五阶梯自适应能效守护系统]
                        - 超充沛/待机静默期: 300s
                        - 平稳消耗期: 150s
                        - 警惕消耗期: 60s
                        - 临界熔断区: 25s
                        - 实时消耗速率 (%/min) 与 ETA
```

### 1. 双周期智能调度评分算法 (Dual-Horizon Quota Scoring)

工具告别单一配额维度的简陋判断，引入**双周期动态加权评分模型**：

$$\text{Score} = 0.70 \times \text{Quota}_{5h} + 0.30 \times \text{Quota}_{week}$$

- **短周期爆发权重 (70%)**：优先保障当前开发会话具备充裕的即时配额。
- **长周期周限额权重 (30%)**：防止过早透支每周总配额，实现账号池在 7 天周期内的均衡负载。
- **15% 临界熔断避让 (Safety Cut-off)**：当当前活动账号的任一核心模型配额跌破 15% 临界线时，守护引擎判定进入高危耗尽阶段，自动在账号池中选出评分最高且状态健康的账号，实施**抢先式平滑热切换**，彻底规避因 429 报错造成的代码生成截断。

### 2. Win32 控制台协同热重载与专属 UUID 独立复原 (Zero-Collision Resumption)

为了在切换凭据后让所有正在运行的 `agy` 终端无缝载入新凭据，系统采用 Win32 原生控制台底层交互：

1. **协同优雅中断**：通过 `AttachConsole` 与 `WriteConsoleInput` 向目标外部终端注入 `Ctrl+C` 与回车确认序列，让 `agy` 优雅持久化当前会话状态并安全退出。
2. **祖先进程链防御机制**：通过 `CreateToolhelp32Snapshot` 回溯进程树，自动识别并严格保护当前执行操作的终端自身，杜绝“误杀自己”的尴尬现象。
3. **双层 UUID 嗅探引擎**：
   - 第一层：读取 Win32 控制台屏幕缓冲区字符流，正则解析输出的 `Conversation ID: <UUID>`；
   - 第二层：扫描本地运行时日志 (`~/.gemini/antigravity-cli/brain/`)，建立 PID 与专属 UUID 的物理映射。
4. **独占式无冲突复原**：重载终端时不再盲目执行 `agy -c`，而是精准下发 `agy --conversation=<专属UUID>`，确保无论同时打开多少个终端窗口，每个窗口均精确恢复各自的历史上下文，**100% 杜绝并发互斥锁死**。

### 3. 五阶梯自适应动态调频与消耗速率估算 (Adaptive Polling & Burn-Rate)

为了在“高敏度防熔断”与“极致能效/零资源浪费”之间取得完美平衡，内置守护引擎采用五阶梯自适应轮询机制：

| 配额状态区间 | 运行态判定条件 | 轮询周期 | 设计目标 |
| :--- | :--- | :--- | :--- |
| **超充沛安全区** | 核心模型配额均 $> 75\%$ | **300 秒 (5 分钟)** | 极致能效，杜绝无意义的网络请求与 CPU 唤醒 |
| **待机静默期** | 系统中未检测到活动的 `agy` 终端 | **300 秒 (5 分钟)** | 开发者暂停编码时，系统降频休眠 |
| **平稳消耗区** | 配额处于 $35\% \sim 75\%$ | **150 秒 (2.5 分钟)** | 兼顾能效与常规状态监控 |
| **警惕消耗区** | 配额处于 $15\% \sim 35\%$ | **60 秒 (1 分钟)** | 密切追踪消耗速度，准备候选切换队列 |
| **临界熔断区** | 任一配额 $< 15\%$ | **25 秒** | 高敏度捕捉熔断切号时机 |

- **动态配额消耗速率探测**：系统在监控过程中自动根据历史采样点计算每分钟配额消耗率（`%/min`），并实时推算当前配额的预计耗尽时间（ETA），提供精准的可视化预警。

### 4. 沉睡周额度时钟预热激活器 (Weekly Clock Pre-warm Kickstart)

- **机制洞察**：Google Antigravity 中 Claude 模型的 7 天周限额倒计时，仅在账号发出首个有效请求时才由服务端真正激活。
- **一键预热**：通过内置的 `prewarm` 指令（或选单 `[P]`），程序会并发遍历账号池中的每个账号。若检测到某账号的周时钟仍处于沉睡状态，则发送极简轻量级探测帧（Minimal Token Probe）。
- **零损耗红利**：在几乎不消耗任何可用额度的情况下，立即激活该账号的 7 天周时钟倒计时。让备用账号“人在阵中坐，时钟自然走”，轮到使用时早已拥有充沛且持续滚动的配额。

### 5. 全局默认模型无缝切换系统 (Global Default Model Switcher)

无需进入每个终端单独敲配置指令，配额中心直接统管全局默认模型：
- **Claude Sonnet 4.6 (Thinking)**：深度代码重构、架构设计与高难度逻辑推理首选。
- **Gemini 3.8 Flash (High)**：超高吞吐、毫秒级响应、日常开发与批量脚本编写首选。
- **Gemini 3.1 Pro (High)**：综合多模态与长上下文复杂任务平衡之选。

一键切换后，全局配置文件与环境变量即时同步，新建会话立即可用。

### 6. 后台脱机守护模式 (Headless Guard Daemon) & Windows Toast 气泡

- **后台常驻运行**：执行 `guard --bg` 后，程序通过 Windows `CREATE_NO_WINDOW | DETACHED_PROCESS` 标志脱离终端静默驻留，不占用任何任务栏图标或黑窗口。
- **原生 Toast 通知**：当触发 15% 自动熔断切号、检测到凭据失效或重载终端时，直接调用 Windows 10/11 原生 Toast 气泡向桌面发送通知，开发人员无需分心即可洞悉系统状态。
- **进程互斥锁与生命周期管理**：严格的单实例 PID 锁管理，支持 `--status` 状态探测与 `--stop` 优雅停机。

### 7. 合规性与系统级凭据安全 (Compliance & Credential Security)

- **严格遵循 Google 服务条款**：
  - 本工具**不破解、不篡改、不反代**任何 Google 官方传输协议与客户端二进制。
  - 本工具未搭建任何中心化转发服务器，所有 API 请求均为开发者本地机器与 Google 官方服务器之间的标准直接通信。
  - 本工具严禁用于自动化批量注册机或滥用规避。
- **Windows 原生凭据保护**：所有活跃凭据直接写入 Windows 凭据管理器（Credential Manager: `gemini:antigravity`），享受操作系统内核级 DPAPI 强加密隔离保护。

---

## 三、交互式终端界面展示

### 1. 主控制台交互菜单 (CLI Interactive Menu)

直接运行 `AGY多账号配额中心.exe` 即可进入全功能交互控制台：

```text
======================================================================
 >>> Antigravity (AGY) 多账号与配额管理中心 v2.3.0 (Rust) <<<
======================================================================
 当前活动账号: dev-primary@gmail.com  |  套餐类型: Antigravity
 系统本地时间: 2026-09-06 12:30:00
----------------------------------------------------------------------
 [1] 查看当前账号额度详情 (Detailed Quota)
 [2] 查看所有账号全局大盘 (All Accounts Overview)
 [3] 切换当前活动账号 (Switch Account - 交互选择 / 序号 / 邮箱)
 [4] 智能切至最高额度账号 (Auto-Switch to Best Quota Account)
 [A] 后台自动调配守护服务 (Auto-Guard Daemon - 监控/自动切号/通知)
 [P] 一键唤醒全账号池沉睡周额度时钟 (Pre-warm Weekly Clocks - 提前激活7天倒计时)
 [M] 切换全局默认模型 (Switch Default Model: Claude / Gemini)
 [5] 保存当前 agy 账号至账号池 (Save Active agy Account)
 [6] 添加新账号到账号池 (Add New Account - 浏览器授权 / Token)
 [7] 从账号池移除账号 (Remove Account)
 [8] 强制刷新当前账号凭据 (Refresh Token)
 [9] 自动重启所有运行中的 agy 终端会话 (Reload All Sessions)
 [0] 退出程序 (Exit)
======================================================================
>> 请输入选项 [0-9/A/P/M]: 
```

### 2. 全局多账号配额大盘看板 (All Accounts Overview)

```text
================================================================================================
 账号全局额度总览大盘 (AGY Account Pool Monitor)
================================================================================================
#   状态       邮箱账号                         Gemini 5h          Gemini 周           Claude 5h          Claude 周          
---------------------------------------------------------------------------------------------------
[0] [当前*]   dev-primary@gmail.com        [██████████]  100% [██████████]  100% [██████████]  100% [██████████]  100% (6天23h)
[1] [推荐★]   dev-backup@gmail.com         [██████████]  100% [██████████]  100% [██████████]  100% [██████████]  100% (6天23h)
[2] [健康]     team-shared@company.com      [████████░░]   82% [█████████░]   91% [██████░░░░]   60% [████████░░]   85% (3天14h)
---------------------------------------------------------------------------------------------------
* 账号池统计: 共 3 个账号 | 正常可用: 3 | 需授权/受限: 0
* 推荐说明: [推荐★] 综合双周期可用度最高 (Score: 100.0)
```

### 3. 一键切换与终端零冲突复原反馈 (Switch & Resumption Output)

```text
[*] 检测到 2 个运行中的 agy 终端会话，正在下发协同热重载退出与继续...
[已捕获] 终端 (PID: 12040) 会话 ID: 3a1f4b82-82ec-498c-bb05-09ceb893f019
[已就绪] 终端 (PID: 12040) 成功恢复独占会话上下文。
[已就绪] 终端 (PID: 18452) 成功恢复独占会话上下文。

[+] 账号切换成功！当前活动账号已切换为: dev-backup@gmail.com
凭据已更新至系统凭据管理器与本地配置。
[成功] 已成功向 2 个外部 agy 终端下发热重载与会话复原指令！
```

---

## 四、CLI 命令行指令速查 (Command Reference)

除了控制台交互菜单，本工具原生支持完整的一级与二级 CLI 命令行直接调用，方便嵌入批处理、快捷方式或外部脚本：

```bash
# 查看帮助文档
AGY多账号配额中心.exe --help

# 1. 额度与状态查询
AGY多账号配额中心.exe usage              # 输出所有账号的配额对比 ASCII 大盘
AGY多账号配额中心.exe accounts           # 列出账号池内所有已保存的账号列表

# 2. 账号切换与智能调度
AGY多账号配额中心.exe switch <序号/邮箱> # 切换到指定序号或邮箱账号，并自动协同重载外部终端
AGY多账号配额中心.exe auto               # 智能计算双周期评分，切至最优账号并自动协同重载
AGY多账号配额中心.exe save               # 保存当前正在使用的 agy 活动账号快照到账号池

# 3. 守护服务管理 (Auto-Guard Daemon)
AGY多账号配额中心.exe guard              # 前台启动守护监听模式 (控制台显示动态仪表盘，Ctrl+C 退出)
AGY多账号配额中心.exe guard --bg         # 后台静默启动守护服务 (无窗口常驻运行)
AGY多账号配额中心.exe guard --status     # 查询后台守护服务的运行状态与 PID
AGY多账号配额中心.exe guard --stop       # 停止后台运行的守护服务

# 4. 沉睡周额度时钟预热
AGY多账号配额中心.exe prewarm            # 遍历全账号池，唤醒所有处于沉睡状态的 Claude 7 天时钟

# 5. 全局默认模型切换
AGY多账号配额中心.exe model              # 交互式选择默认模型
AGY多账号配额中心.exe model claude       # 快捷设为 Claude Sonnet 4.6 (Thinking)
AGY多账号配额中心.exe model flash        # 快捷设为 Gemini 3.8 Flash (High)
AGY多账号配额中心.exe model pro          # 快捷设为 Gemini 3.1 Pro (High)

# 6. 会话重载辅助
AGY多账号配额中心.exe reload             # 手动探测所有活动 agy 终端并下发会话独立复原热重载
```

---

## 五、源码编译与本地安装 (Build & Install)

### 编译环境要求
- **操作系统**：Windows 10 / 11 64-bit (x86_64)
- **开发工具**：[Rust](https://rustup.rs/) (1.75+ 推荐), MSVC 编译器套件 (`Visual Studio Build Tools`)

### 编译步骤

```bash
# 克隆仓库
git clone https://github.com/moxiuren/agy-quota-center.git
cd agy-quota-center/src-rust

# 编译极限尺寸与性能优化的 Release 二进制
cargo build --release
```

编译生成的文件位于 `src-rust/target/release/agy-quota-center.exe`（单文件约 1.8MB）。可将其重命名为 `AGY多账号配额中心.exe`，拷贝至桌面或加入系统 `PATH` 环境变量中随时使用。

---

## 六、常见问题解答 (FAQ)

**Q: 协同热重载退出后，我的代码会丢失吗？**
> **A:** 绝不会。本工具下发的 `Ctrl+C` 属于官方支持的优雅退出信号，`agy` 在退出时会自动将当前的执行树、对话记录完整写入本地 SQLite / JSONL 存储。随后系统利用获取到的独占 UUID 重启会话，所有历史上下文完好如初。

**Q: 为什么后台守护服务推荐使用 `guard --bg`？**
> **A:** `guard --bg` 会在 Windows 中以独立无窗口进程运行，内存开销仅约 10MB，平时处于 300 秒长休眠模式，CPU 占用几乎为 0%。一旦遇到额度低于 15% 临界点或切换事件，自动通过桌面 Toast 气泡提醒，不打扰全屏编程。

**Q: 唤醒 Claude 周额度时钟会不会浪费我的额度？**
> **A:** 不会。预热器采用的是轻量探测机制，仅向服务端验证时钟上下文，不生成冗余文本，对实际额度的消耗接近于 0。

---

## 七、免责声明 (Disclaimer)

1. **学习与研究目的**：本项目仅供个人开发者在本地学习 Rust 语言系统编程、Windows 进程与控制台 API 交互技术，以及方便管理个人合法拥有的多账号凭据使用。
2. **非官方工具声明**：本项目属于非官方开源开发辅助工具，与 Google 或 Anthropic 官方不存在从属、赞助或背书关系。
3. **服务条款遵守**：使用者应当严格遵守 Google 服务条款以及 Generative AI 相关使用政策与规范。严禁将本工具用于商业黑产、自动化机刷、多账号恶意套利或反向代理搭建。因使用者不当使用导致的任何账号封控或经济损失，由使用者自行承担，与本项目贡献者无关。

---

## 八、开源许可 (License)

本项目采用 [MIT 许可证](LICENSE) 开源，欢迎提交 Issue 与 Pull Request 共同改进！
