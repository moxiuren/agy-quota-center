#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Antigravity Account Manager CLI (agy-account)
多账号管理、全局额度大盘与账号池维护工具
"""

import sys
import argparse
from concurrent.futures import ThreadPoolExecutor
from agy_manager import (
    list_all_accounts, fetch_account_quota, parse_quota_buckets,
    calculate_effective_quota, switch_to_account, save_current_agy_to_pool,
    remove_account, pad_visual, format_compact_countdown,
    C_RESET, C_BOLD, C_DIM, C_GREEN, C_YELLOW, C_RED, C_CYAN, C_MAGENTA, C_WHITE
)

def format_pct_cell(val):
    if val is None:
        return f"{C_DIM} --  {C_RESET}"
    pct = val * 100
    if pct >= 50:
        return f"{C_GREEN}{pct:4.0f}%{C_RESET}"
    elif pct >= 20:
        return f"{C_YELLOW}{pct:4.0f}%{C_RESET}"
    else:
        return f"{C_RED}{pct:4.0f}%{C_RESET}"

def list_accounts_dashboard():
    print(f"\n{C_BOLD}{C_CYAN}=============================================================================================={C_RESET}")
    print(f"{C_BOLD} >>> Antigravity 多账号全局额度总览大盘 (AGY Account Pool Monitor) <<<{C_RESET}")
    print(f"{C_BOLD}{C_CYAN}=============================================================================================={C_RESET}")
    print(f"{C_DIM}正在并发同步各账号最新配额状态...{C_RESET}\n")

    accounts, current_id = list_all_accounts()
    if not accounts:
        print(f"{C_YELLOW}账号池为空。你可以通过 `agy-account save` 将当前运行中的 agy 账号存入账号池。{C_RESET}\n")
        return

    def worker(acc):
        summary, err = fetch_account_quota(acc)
        parsed = parse_quota_buckets(summary) if summary else None
        eff = calculate_effective_quota(parsed)
        return acc, summary, err, parsed, eff

    with ThreadPoolExecutor(max_workers=5) as executor:
        results = list(executor.map(worker, accounts))

    # Find best candidate
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

    header = f" {pad_visual('#', 4)} {pad_visual('状态', 8)} {pad_visual('邮箱账号', 24)} {pad_visual('Gemini (5h/周)', 15)} {pad_visual('G-5h刷新', 10)} {pad_visual('Claude (5h/周)', 15)} {pad_visual('C-5h刷新', 10)}"
    print(header)
    print(f"{'-' * 94}")

    normal_count = 0
    forbidden_count = 0
    nearest_reset_email = None
    nearest_reset_sec = float('inf')
    nearest_reset_info = ""

    from datetime import datetime, timezone
    import re

    for idx, (acc, summary, err, parsed, eff) in enumerate(results):
        is_cur = (acc['id'] == current_id)
        is_best = (best_acc and acc['id'] == best_acc['id'] and not is_cur)

        if is_cur:
            status = f"{C_GREEN}[当前*]{C_RESET}"
            normal_count += 1
        elif is_best:
            status = f"{C_MAGENTA}[推荐*]{C_RESET}"
            normal_count += 1
        elif err and "403" in err:
            status = f"{C_RED}[无许可]{C_RESET}"
            forbidden_count += 1
        elif err:
            status = f"{C_YELLOW}[异常]{C_RESET}"
            forbidden_count += 1
        else:
            status = f"{C_CYAN}[可用]{C_RESET}"
            normal_count += 1

        email = acc['email']
        if is_cur:
            email_str = f"{C_BOLD}{C_WHITE}{email}{C_RESET}"
        elif is_best:
            email_str = f"{C_MAGENTA}{email}{C_RESET}"
        else:
            email_str = email

        if parsed:
            g_5h = format_pct_cell(parsed.get('gemini_5h'))
            g_w = format_pct_cell(parsed.get('gemini_weekly'))
            c_5h = format_pct_cell(parsed.get('claude_5h'))
            c_w = format_pct_cell(parsed.get('claude_weekly'))
            g_str = f"{g_5h} / {g_w}"
            c_str = f"{c_5h} / {c_w}"
            g_cd = format_compact_countdown(parsed.get('gemini_5h_reset'), parsed.get('gemini_5h'))
            c_cd = format_compact_countdown(parsed.get('claude_5h_reset'), parsed.get('claude_5h'))

            # Check nearest upcoming reset among depleted accounts
            for (rt, frac, m_type) in [(parsed.get('gemini_5h_reset'), parsed.get('gemini_5h'), 'Gemini 5h'),
                                       (parsed.get('claude_5h_reset'), parsed.get('claude_5h'), 'Claude 5h')]:
                if rt and frac is not None and frac < 0.999:
                    try:
                        clean_iso = re.sub(r'\.\d+', '', rt).replace('Z', '+00:00')
                        t_dt = datetime.fromisoformat(clean_iso)
                        diff_sec = (t_dt - datetime.now(timezone.utc)).total_seconds()
                        if 0 < diff_sec < nearest_reset_sec:
                            nearest_reset_sec = diff_sec
                            nearest_reset_email = email
                            nearest_reset_info = f"{m_type} 将于 {format_compact_countdown(rt, frac)} 恢复"
                    except Exception:
                        pass
        else:
            g_str = f"{C_DIM} --  /  -- {C_RESET}"
            c_str = f"{C_DIM} --  /  -- {C_RESET}"
            g_cd = f"{C_DIM}--{C_RESET}"
            c_cd = f"{C_DIM}--{C_RESET}"

        row = f" {pad_visual(f'[{idx}]', 4)} {pad_visual(status, 8)} {pad_visual(email_str, 24)} {pad_visual(g_str, 15)} {pad_visual(g_cd, 10)} {pad_visual(c_str, 15)} {pad_visual(c_cd, 10)}"
        print(row)

    print(f"{'-' * 94}")
    print(f"[统计] 账号池: 共 {len(results)} 个账号 | 正常可用: {normal_count} | 需授权/受限: {forbidden_count}")
    if nearest_reset_email:
        print(f"[*] 最近刷新: {C_BOLD}{C_CYAN}{nearest_reset_email}{C_RESET} 的 {C_BOLD}{C_YELLOW}{nearest_reset_info}{C_RESET}！")
    if best_acc:
        print(f"[*] 智能推荐: 可用额度最充沛账号为 {C_BOLD}{C_MAGENTA}{best_acc['email']}{C_RESET}，运行 `agy-switch auto` 一键切换！")
    print(f"{C_DIM}[说明]: 生图功能 (Imagen 3 / gemini-3.1-flash-image) 共享 Gemini 5h/周配额，查看 Gemini 额度即代表生图额度。{C_RESET}")
    print(f"{C_DIM}[提示]: 输入 `agy-switch <序号>` 即可快速切换当前活动账号。{C_RESET}\n")

def main():
    parser = argparse.ArgumentParser(
        description="Antigravity 多账号管理 CLI (agy-account)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
子命令与用法:
  agy-account                    显示账号池全局额度看板
  agy-account list               同上
  agy-account switch <编号/邮箱>  一键切换当前活动账号 (如: agy-account switch 0)
  agy-account auto               自动切换至额度最充沛的账号
  agy-account save [别名]        将当前正在使用的 agy 账号凭据保存/快照至账号池
  agy-account remove <编号/邮箱>  从账号池中移除指定账号
        """
    )
    subparsers = parser.add_subparsers(dest="action", help="操作指令")

    # list
    subparsers.add_parser("list", help="查看所有账号的实时额度状态")

    # switch
    p_switch = subparsers.add_parser("switch", help="切换当前活动账号")
    p_switch.add_argument("target", help="目标账号编号 (0, 1...)、邮箱关键字或 'auto'")

    # auto
    subparsers.add_parser("auto", help="自动切至额度最充沛账号")

    # save
    p_save = subparsers.add_parser("save", help="保存当前 agy 凭据至账号池")
    p_save.add_argument("name", nargs="?", default=None, help="账号别名（可选）")

    # remove
    p_remove = subparsers.add_parser("remove", help="从账号池移除账号")
    p_remove.add_argument("target", help="要移除的账号编号或邮箱")

    args = parser.parse_args()

    if not args.action or args.action == "list":
        list_accounts_dashboard()
    elif args.action == "switch":
        if args.target.lower() in ['auto', 'best']:
            from agy_switch import get_accounts_with_quota
            _, _, best_acc = get_accounts_with_quota()
            if best_acc:
                target = best_acc['id']
            else:
                print(f"{C_RED}[-] 未能找到额度可用且非当前的推荐账号。{C_RESET}")
                return
        else:
            target = args.target
        success, msg = switch_to_account(target)
        if success:
            print(f"{C_BOLD}{C_GREEN}[+] 账号切换成功！当前活动账号已切换为: {msg}{C_RESET}")
        else:
            print(f"{C_BOLD}{C_RED}[-] 切换失败: {msg}{C_RESET}", file=sys.stderr)
            sys.exit(1)
    elif args.action == "auto":
        from agy_switch import get_accounts_with_quota
        _, _, best_acc = get_accounts_with_quota()
        if best_acc:
            success, msg = switch_to_account(best_acc['id'])
            if success:
                print(f"{C_BOLD}{C_GREEN}[+] 自动切换成功！已切至最充沛账号: {msg}{C_RESET}")
            else:
                print(f"{C_BOLD}{C_RED}[-] 自动切换失败: {msg}{C_RESET}", file=sys.stderr)
                sys.exit(1)
        else:
            print(f"{C_RED}[-] 未能找到额度可用且非当前的推荐账号。{C_RESET}")
    elif args.action == "save":
        success, msg = save_current_agy_to_pool(args.name)
        if success:
            print(f"{C_BOLD}{C_GREEN}[+] 成功将当前账号 [{msg}] 保存至多账号池！{C_RESET}")
        else:
            print(f"{C_BOLD}{C_RED}[-] 保存失败: {msg}{C_RESET}", file=sys.stderr)
            sys.exit(1)
    elif args.action == "remove":
        success, msg = remove_account(args.target)
        if success:
            print(f"{C_BOLD}{C_GREEN}[+] 成功移除账号: {msg}{C_RESET}")
        else:
            print(f"{C_BOLD}{C_RED}[-] 移除失败: {msg}{C_RESET}", file=sys.stderr)
            sys.exit(1)

if __name__ == "__main__":
    main()
