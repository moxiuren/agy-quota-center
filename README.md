# AGY 多账号配额中心 (AGY Quota Center)

[![Release](https://img.shields.io/github/v/release/moxiuren/agy-quota-center?style=flat-square&color=blue)](https://github.com/moxiuren/agy-quota-center/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows_x64-0078D6?style=flat-square&logo=windows)](https://microsoft.com)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

> 面向 Google Antigravity (`agy`) CLI 的多账号配额管理与平滑热重载工具。提供配额自动监控切号、跨终端会话无缝恢复、周限额时钟预热与全局默认模型切换。

---

## 项目简介

在使用 Google Antigravity (`agy`) 命令行工具进行开发时，单个账号的可用配额经常面临耗尽风险。手动切换账号通常需要终止所有正在运行的终端会话并重新登录，极易打断工作流程。

**AGY Quota Center** 是专为此场景开发的原生管理工具：
- **自动检测与切号**：当当前账号配额告急时，自动在账号池中选择额度最充沛的账号进行切换，避免因 429 报错中断开发；
- **跨终端无感恢复**：切换账号后，自动向运行中的终端下发重载指令，完整保留正在进行的对话上下文；
- **周额度时钟预热**：针对 Claude 7 天限额机制，提前激活未调用账号的刷新倒计时；
- **绿色单文件**：纯 Rust 构建（约 1.7MB），无 Python 或第三方环境依赖，解压即用。

---

## 核心特性

- **智能配额熔断与自动切换**：持续监控 5 小时与每周配额余量。当活跃账号配额降至 15% 临界阈值时，自动触发抢先式切号。
- **跨终端协同热重载 (Zero-Collision)**：通过 Win32 控制台输入接口实现优雅重启，精准提取会话专属 UUID，多窗口并发运行互不冲突。
- **周配额时钟预热唤醒 (Pre-warm)**：通过轻量探针请求，激活账号池内沉睡的 Claude 7 天重置倒计时，确保备用账号随时处于滚动刷新状态。
- **全局默认模型无缝切换**：支持在 Claude Sonnet 4.6 (Thinking)、Claude Opus 4.6 (Thinking)、Gemini 3.8 Flash (High) 与 Gemini 3.1 Pro (High) 之间即时切换。
- **全账号配额总览看板**：直观展示账号池内各账号的 Gemini 与 Claude 额度比例及具体刷新倒计时。
- **后台脱机守护服务**：支持后台无窗口常驻运行，状态变更自动通过 Windows 原生通知提醒。

---

## 快速上手

### 1. 获取程序
从 [GitHub Releases](https://github.com/moxiuren/agy-quota-center/releases/latest) 下载最新版本的可执行文件：
- **`AGY_Quota_Center_v2.3.0.exe`**

下载后放置于任意目录或桌面，直接运行即可。

### 2. 控制台交互菜单

启动程序后进入交互式控制台主界面：

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

### 3. 核心功能速查表

| 操作需求 | 菜单输入 | 功能说明 |
| :--- | :---: | :--- |
| **查询所有账号配额** | `2` | 集中展示全部账号的可用百分比与刷新倒计时 |
| **切至最高额度账号** | `4` | 自动计算综合评分最优账号并切换，协同重载终端 |
| **启动自动守护服务** | `A` | 开启后台自动监控，配额不足时自动切换并通过系统通知提醒 |
| **激活周额度时钟** | `P` | 批量唤醒账号池内尚未启动 7 天倒计时的账号 |
| **切换全局默认模型** | `M` | 选择 Claude (Sonnet / Opus) 或 Gemini 作为全局默认调用模型 |
| **导入新 Google 账号** | `6` | 支持浏览器授权或粘贴 Refresh Token 录入账号池 |

---

## 命令行调用 (CLI Usage)

程序支持通过命令行参数直接调用，便于集成至批处理或快捷方式：

```bash
# 查看所有账号配额概览
AGY_Quota_Center_v2.3.0.exe usage

# 自动切至最优额度账号并重载终端
AGY_Quota_Center_v2.3.0.exe auto

# 后台静默启动守护服务 (无控制台窗口)
AGY_Quota_Center_v2.3.0.exe guard --bg

# 查询守护服务运行状态
AGY_Quota_Center_v2.3.0.exe guard --status

# 停止后台守护服务
AGY_Quota_Center_v2.3.0.exe guard --stop

# 批量预热沉睡的周配额时钟
AGY_Quota_Center_v2.3.0.exe prewarm

# 切换全局默认模型 (claude / opus / flash / pro)
AGY_Quota_Center_v2.3.0.exe model opus
AGY_Quota_Center_v2.3.0.exe model claude
```

---

## 常见问题 (FAQ)

**1. 切换账号是否会导致正在进行的对话和代码丢失？**
> 不会。程序向终端发送优雅退出信号，Antigravity 会自动将当前执行状态与会话记录持久化至本地存储；切换完成后通过专属会话 ID 重新拉起，对话上下文完整恢复。

**2. 预热周额度时钟是否会消耗可用配额？**
> 几乎不消耗。预热模块使用轻量级探针请求仅用于打点触发服务端的周配额计时器，不生成冗余输出，单次调用额度消耗小于 0.2%。

**3. 凭据存储与使用是否安全？**
> 本工具属于本地运行的辅助管理程序，不架设任何中间转发服务。所有凭据均存储于本地 Windows 凭据管理器（Credential Manager: `gemini:antigravity`）中，采用操作系统级 DPAPI 加密保护。

---

## 开源协议

本项目基于 [MIT License](LICENSE) 开源。
