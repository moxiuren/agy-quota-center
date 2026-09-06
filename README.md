# AGY 多账号配额中心 (AGY Quota Center)

[![Release](https://img.shields.io/github/v/release/moxiuren/agy-quota-center?style=flat-square&color=blue)](https://github.com/moxiuren/agy-quota-center/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows_x64-0078D6?style=flat-square&logo=windows)](https://microsoft.com)
[![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

> **一句话介绍**：让你的 Google Antigravity (`agy`) 写代码永不中断！自动换号、无感恢复会话、提前激活 7 天额度刷新、全账号额度一目了然。

---

## 什么是 AGY 配额中心？

在使用 Google Antigravity (`agy`) 辅助写代码时，最痛苦的事情就是**额度突然用光**。以往你必须关掉所有黑窗口、重新登录别的 Google 账号，再重新打开终端，心流瞬间被打断。

**AGY 配额中心** 就是为了解决这个问题而设计的小工具：
- 它可以**一键甚至全自动**帮你把额度切到最充沛的备用账号；
- 换号时**不需要关窗口**，它会自动刷新你正在运行的终端，写到一半的会话完好无损地继续；
- 绿色单文件（约 1.7MB），下载双击直接用，不需要装任何复杂的 Python 环境。

---

## 核心功能（大白话版）

### 1. 额度用光前，自动帮你切号
- 内置后台守护模式。当当前账号的额度掉到 15% 临界点时，它会自动挑出你账号池里最充沛的账号换上去。
- 彻底告别写代码写到一半弹出 `429 Too Many Requests`（额度耗尽）的尴尬。

### 2. 终端无感热重载，代码对话不丢失
- 自动帮你在后台通知所有开着的 `agy` 窗口换上新账号。
- 每个窗口各自恢复各自的历史对话，绝不冲突、不卡死、不丢失记录。

### 3. 一键激活 7 天刷新倒计时（很多人不知道的暗坑）
- **官方机制**：Claude 模型的一星期限额，如果你登录后没在终端跟它说话，系统**根本不会开始跑 7 天倒计时**！
- 本工具提供一键唤醒功能，用几乎不掉额度的极微量探测，帮你把所有备用账号的 7 天刷新时钟提前跑起来。等你需要用的时候，额度早已自然回满。

### 4. 一键切换默认模型
- 随心在 **Claude Sonnet 4.6**、**Gemini 3.8 Flash** 和 **Gemini 3.1 Pro** 之间秒切，不用每次敲复杂长指令。

### 5. 账号额度一目了然
- 一张表看清所有账号的 Gemini / Claude 5小时限额与每周限额，还能精确显示还有几小时、几天刷新。

---

## 新手 3 步快速上手

### 第 1 步：下载程序
前往 [GitHub Releases 页面](https://github.com/moxiuren/agy-quota-center/releases/latest)，下载最新的：
👉 **`AGY_Quota_Center_v2.3.0.exe`**

下载后放在桌面上，可以直接双击运行，也可以重命名为任意你喜欢的名字。

---

### 第 2 步：双击打开，看懂主菜单

双击打开后会看到一个清晰的控制台选单：

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

---

### 第 3 步：最常用的几个操作

| 想做的事情 | 怎么操作 | 效果说明 |
| :--- | :--- | :--- |
| **看所有账号还剩多少额度** | 输入 `2` 回车 | 打印出全部账号的 Gemini 和 Claude 剩余额度与倒计时 |
| **谁额度多就换谁** | 输入 `4` 回车 | 自动计算最优账号，秒切过去，并帮所有窗口刷新好 |
| **想开启无人值守（自动换号）** | 输入 `A` 回车 | 开启后台守护。只要额度快没了它就会自动换号，右下角弹窗通知你 |
| **让所有备用号的 7 天倒计时跑起来** | 输入 `P` 回车 | 自动激活所有号的周刷新时钟 |
| **换模型 (Claude / Gemini)** | 输入 `M` 回车 | 快速选择你想要的默认大模型 |
| **存入新的 Google 账号** | 输入 `6` 回车 | 支持弹出浏览器一键登录或者粘贴 Token 添加新账号 |

---

## 进阶：命令行直接调用（可选）

如果你喜欢敲命令或写批处理脚本，可以直接传参数运行：

```bash
# 1. 打印所有账号的额度大盘
AGY_Quota_Center_v2.3.0.exe usage

# 2. 自动切换到额度最高的账号并刷新终端
AGY_Quota_Center_v2.3.0.exe auto

# 3. 后台静默启动自动守护服务（无黑窗口，右下角气泡通知）
AGY_Quota_Center_v2.3.0.exe guard --bg

# 4. 激活所有备用账号的 7 天刷新时钟
AGY_Quota_Center_v2.3.0.exe prewarm

# 5. 快速把默认模型换为 Claude
AGY_Quota_Center_v2.3.0.exe model claude
```

---

## 常见疑问 (FAQ)

**Q：切换账号的时候，我写到一半的代码和对话记录会丢失吗？**
> **A：完全不会。** 本工具采用优雅退出指令，`agy` 会把当前上下文完整保存在本地数据库中，换完号后会自动用你原本的会话 ID 重新打开，对话和代码完全都在。

**Q：这算不算外挂？会不会封号？**
> **A：完全合规安全。**
> 1. 本工具没有破坏、破解或篡改任何官方程序，也没有搭建任何第三方转发服务器。
> 2. 它只是一个在你自己 Windows 电脑上管理你合法拥有的多个 Google 账号的辅助脚本。
> 3. 所有账号凭据都直接存放在 Windows 系统自带的凭据管理器里（受到 Windows 底层加密保护）。

**Q：后台守护进程占电脑资源吗？**
> **A：几乎为 0。** 工具采用纯 Rust 原生编写，在额度充沛或你没在写代码时，会进入长达 5 分钟的静默休眠，内存占用仅约 10MB，CPU 占用 0%。

---

## 开源协议

本项目基于 [MIT License](LICENSE) 开源。欢迎 Star 或提交 Issue！
