#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Antigravity Quick Switch CLI (agy-switch)
一键秒切 Antigravity / AGY 账号
"""

import os
import sys
import argparse
from concurrent.futures import ThreadPoolExecutor

# Force Windows Console to UTF-8
if sys.platform == 'win32':
    try:
        import ctypes
        ctypes.windll.kernel32.SetConsoleOutputCP(65001)
        ctypes.windll.kernel32.SetConsoleCP(65001)
    except Exception:
        pass

if hasattr(sys.stdout, 'reconfigure'):
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass
if hasattr(sys.stderr, 'reconfigure'):
    try:
        sys.stderr.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass

from agy_manager import (
    list_all_accounts, fetch_account_quota, parse_quota_buckets,
    calculate_effective_quota, switch_to_account, pad_visual, format_compact_countdown,
    C_RESET, C_BOLD, C_DIM, C_GREEN, C_YELLOW, C_RED, C_CYAN, C_MAGENTA, C_WHITE
)

def format_pct(val):
    if val is None:
        return f"{C_DIM} -- {C_RESET}"
    pct = val * 100
    if pct >= 50:
        return f"{C_GREEN}{pct:4.0f}%{C_RESET}"
    elif pct >= 20:
        return f"{C_YELLOW}{pct:4.0f}%{C_RESET}"
    else:
        return f"{C_RED}{pct:4.0f}%{C_RESET}"

def get_accounts_with_quota():
    accounts, current_id = list_all_accounts()
    if not accounts:
        return [], current_id, None

    # Fetch quota concurrently
    def worker(acc):
        summary, err = fetch_account_quota(acc)
        parsed = parse_quota_buckets(summary) if summary else None
        eff = calculate_effective_quota(parsed)
        return acc, summary, err, parsed, eff

    with ThreadPoolExecutor(max_workers=5) as executor:
        results = list(executor.map(worker, accounts))

    # Find best candidate (exclude current and 403/errors)
    best_acc = None
    best_score = -1.0
    for acc, summary, err, parsed, eff in results:
        if err or not parsed:
            continue
        if acc['id'] == current_id:
            continue
        if eff > best_score:
            best_score = eff
            best_acc = acc

    # If all others failed, check if current is available
    if not best_acc:
        for acc, summary, err, parsed, eff in results:
            if not err and parsed and eff > best_score:
                best_score = eff
                best_acc = acc

    return results, current_id, best_acc

def display_interactive_menu():
    print(f"\n{C_BOLD}{C_CYAN}=== Antigravity 账号快速切换面板 (AGY Quick Switch) ==={C_RESET}")
    print(f"{C_DIM}正在并发同步各账号最新额度...{C_RESET}\n")

    results, current_id, best_acc = get_accounts_with_quota()
    if not results:
        print(f"{C_RED}[-] 账号池中暂无可用账号。请使用 `agy-account save` 将当前账号存入池中。{C_RESET}")
        return

    header = f" {pad_visual('#', 4)} {pad_visual('状态', 8)} {pad_visual('邮箱账号', 24)} {pad_visual('Gemini (5h/周)', 15)} {pad_visual('G-5h刷新', 10)} {pad_visual('Claude (5h/周)', 15)} {pad_visual('C-5h刷新', 10)}"
    print(header)
    print(f"{'-' * 94}")

    for idx, (acc, summary, err, parsed, eff) in enumerate(results):
        is_cur = (acc['id'] == current_id)
        is_best = (best_acc and acc['id'] == best_acc['id'] and not is_cur)

        if is_cur:
            status = f"{C_GREEN}[当前*]{C_RESET}"
        elif is_best:
            status = f"{C_MAGENTA}[推荐*]{C_RESET}"
        elif err and "403" in err:
            status = f"{C_RED}[无许可]{C_RESET}"
        elif err:
            status = f"{C_YELLOW}[异常]{C_RESET}"
        else:
            status = f"{C_CYAN}[可用]{C_RESET}"

        email = acc['email']
        if is_cur:
            email_display = f"{C_BOLD}{C_WHITE}{email}{C_RESET}"
        elif is_best:
            email_display = f"{C_MAGENTA}{email}{C_RESET}"
        else:
            email_display = email

        if parsed:
            g_str = f"{format_pct(parsed.get('gemini_5h'))} / {format_pct(parsed.get('gemini_weekly'))}"
            c_str = f"{format_pct(parsed.get('claude_5h'))} / {format_pct(parsed.get('claude_weekly'))}"
            g_cd = format_compact_countdown(parsed.get('gemini_5h_reset'), parsed.get('gemini_5h'))
            c_cd = format_compact_countdown(parsed.get('claude_5h_reset'), parsed.get('claude_5h'))
        else:
            g_str = f"{C_DIM} --  /  -- {C_RESET}"
            c_str = f"{C_DIM} --  /  -- {C_RESET}"
            g_cd = f"{C_DIM}--{C_RESET}"
            c_cd = f"{C_DIM}--{C_RESET}"

        row = f" {pad_visual(f'[{idx}]', 4)} {pad_visual(status, 8)} {pad_visual(email_display, 24)} {pad_visual(g_str, 15)} {pad_visual(g_cd, 10)} {pad_visual(c_str, 15)} {pad_visual(c_cd, 10)}"
        print(row)

    print(f"{'-' * 94}")
    if best_acc:
        print(f"提示: 输入 {C_BOLD}{C_MAGENTA}'auto'{C_RESET} 或回车直接切至推荐账号: {C_BOLD}{best_acc['email']}{C_RESET}")

    try:
        choice = input(f"\n>> 请输入要切换的账号编号 [0-{len(results)-1}] 或邮箱关键字 (按 q 退出): ").strip()
    except (KeyboardInterrupt, EOFError):
        print("\n已取消操作。")
        return

    if not choice or choice.lower() in ['auto', 'best']:
        if best_acc:
            do_switch(best_acc['id'])
        else:
            print("未找到可推荐的账号。")
        return

    if choice.lower() in ['q', 'quit', 'exit']:
        print("已取消。")
        return

    do_switch(choice)

def do_switch(target):
    print(f"\n{C_DIM}[*] 正在切换至目标账号...{C_RESET}")
    success, msg = switch_to_account(target)
    if success:
        print(f"{C_BOLD}{C_GREEN}[+] 账号切换成功！当前活动账号已切换为: {msg}{C_RESET}")
        print(f"{C_DIM}凭据已写入 Windows 凭据管理器，下次执行 `agy` 命令时将自动生效。{C_RESET}\n")
    else:
        print(f"{C_BOLD}{C_RED}[-] 切换失败: {msg}{C_RESET}\n", file=sys.stderr)
        sys.exit(1)

def main():
    parser = argparse.ArgumentParser(
        description="Antigravity 账号快速切换 CLI (agy-switch)",
        epilog="""
示例:
  agy-switch             进入交互式菜单（展示实时额度对比并选择）
  agy-switch auto        自动切到额度最充沛的账号
  agy-switch 0           直接切换到序号为 0 的账号
  agy-switch moxiuren    按邮箱模糊关键字切换
        """
    )
    parser.add_argument("target", nargs="?", default=None, help="目标账号编号 (0, 1...)、邮箱关键字或 'auto'")
    args = parser.parse_args()

    if not args.target:
        display_interactive_menu()
    elif args.target.lower() in ['auto', 'best']:
        _, current_id, best_acc = get_accounts_with_quota()
        if best_acc:
            do_switch(best_acc['id'])
        else:
            print(f"{C_RED}[-] 未能找到额度可用且非当前的推荐账号。{C_RESET}")
    else:
        do_switch(args.target)

if __name__ == "__main__":
    main()
