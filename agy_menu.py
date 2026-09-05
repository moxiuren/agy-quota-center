#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Antigravity (AGY) All-in-One Interactive CLI Menu & Tool
集成配额看板、全局对比、极速切换、账号管理于一体的交互式终端入口
"""

import os
import sys
import json
import time
import uuid
import webbrowser
import http.server
import urllib.request
import urllib.parse
from datetime import datetime, timezone
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

# Add project root to sys.path
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from agy_manager import (
    list_all_accounts, fetch_account_quota, parse_quota_buckets,
    calculate_effective_quota, switch_to_account, save_current_agy_to_pool,
    remove_account, refresh_oauth_token, pad_visual, format_compact_countdown,
    save_account, load_account_index, save_account_index,
    OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, USER_AGENT,
    C_RESET, C_BOLD, C_DIM, C_GREEN, C_YELLOW, C_RED, C_CYAN, C_BLUE, C_MAGENTA, C_WHITE
)
from agy_usage import render_display, get_valid_token, call_api
from agy_account import list_accounts_dashboard
from agy_switch import get_accounts_with_quota

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

def pause():
    try:
        input(f"\n{C_DIM}按回车键返回主菜单...{C_RESET}")
    except (KeyboardInterrupt, EOFError):
        pass

def get_current_active_info():
    email = "未知账号"
    tier = "Antigravity"
    home = os.path.expanduser("~")
    ga_file = os.path.join(home, ".gemini", "google_accounts.json")
    if os.path.exists(ga_file):
        try:
            with open(ga_file, 'r', encoding='utf-8') as f:
                email = json.load(f).get('active', email)
        except Exception:
            pass
    return email, tier

def option_view_current_quota():
    clear_screen()
    print(f"\n{C_DIM}[*] 正在加载当前活动账号配额详情...{C_RESET}")
    token = get_valid_token()
    if not token:
        print(f"{C_RED}[-] 未能获取当前活动的 Antigravity 凭证。{C_RESET}")
        pause()
        return

    try:
        summary = call_api('retrieveUserQuotaSummary', token)
        code_assist = None
        try:
            code_assist = call_api('loadCodeAssist', token)
        except Exception:
            pass

        clear_screen()
        # Default clean view: show core groups only
        print(render_display(summary, None, code_assist))
        print(f"{C_DIM}[说明]: 所有 Gemini 模型共享上述同一个配额池，Claude 模型共享另一个配额池。{C_RESET}")

        try:
            sub = input(f"\n>> 按回车返回主菜单 (或输入 {C_BOLD}'v'{C_RESET} 查看展开的具体模型列表): ").strip()
        except (KeyboardInterrupt, EOFError):
            return

        if sub.lower() == 'v':
            clear_screen()
            print(f"\n{C_DIM}[*] 正在加载具体模型映射明细...{C_RESET}")
            user_quota = call_api('retrieveUserQuota', token)
            clear_screen()
            print(render_display(summary, user_quota, code_assist))
            pause()
    except Exception as e:
        print(f"{C_RED}[-] 查询失败: {e}{C_RESET}")
        pause()

def option_view_all_accounts():
    clear_screen()
    list_accounts_dashboard()
    pause()

def option_switch_account():
    clear_screen()
    print(f"\n{C_BOLD}{C_CYAN}=== 快速切换当前活动账号 (Switch Account) ==={C_RESET}")
    print(f"{C_DIM}正在读取各账号额度状态...{C_RESET}\n")

    results, current_id, best_acc = get_accounts_with_quota()
    if not results:
        print(f"{C_RED}[-] 账号池中暂无可用账号。{C_RESET}")
        pause()
        return

    header = f" {pad_visual('#', 4)} {pad_visual('状态', 8)} {pad_visual('邮箱账号', 24)} {pad_visual('Gemini (5h/周)', 15)} {pad_visual('G-5h刷新', 10)} {pad_visual('Claude (5h/周)', 15)} {pad_visual('C-5h刷新', 10)}"
    print(header)
    print(f"{'-' * 94}")

    def fmt_pct(val):
        if val is None: return f"{C_DIM} -- {C_RESET}"
        pct = val * 100
        if pct >= 50: return f"{C_GREEN}{pct:4.0f}%{C_RESET}"
        elif pct >= 20: return f"{C_YELLOW}{pct:4.0f}%{C_RESET}"
        else: return f"{C_RED}{pct:4.0f}%{C_RESET}"

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
            email_str = f"{C_BOLD}{C_WHITE}{email}{C_RESET}"
        elif is_best:
            email_str = f"{C_MAGENTA}{email}{C_RESET}"
        else:
            email_str = email

        if parsed:
            g_str = f"{fmt_pct(parsed.get('gemini_5h'))} / {fmt_pct(parsed.get('gemini_weekly'))}"
            c_str = f"{fmt_pct(parsed.get('claude_5h'))} / {fmt_pct(parsed.get('claude_weekly'))}"
            g_cd = format_compact_countdown(parsed.get('gemini_5h_reset'), parsed.get('gemini_5h'))
            c_cd = format_compact_countdown(parsed.get('claude_5h_reset'), parsed.get('claude_5h'))
        else:
            g_str = f"{C_DIM} --  /  -- {C_RESET}"
            c_str = f"{C_DIM} --  /  -- {C_RESET}"
            g_cd = f"{C_DIM}--{C_RESET}"
            c_cd = f"{C_DIM}--{C_RESET}"

        row = f" {pad_visual(f'[{idx}]', 4)} {pad_visual(status, 8)} {pad_visual(email_str, 24)} {pad_visual(g_str, 15)} {pad_visual(g_cd, 10)} {pad_visual(c_str, 15)} {pad_visual(c_cd, 10)}"
        print(row)

    print(f"{'-' * 94}")
    if best_acc:
        print(f"提示: 输入 {C_BOLD}{C_MAGENTA}'auto'{C_RESET} 自动切至最高额度账号: {C_BOLD}{best_acc['email']}{C_RESET}")

    try:
        choice = input(f"\n>> 请输入要切换的账号序号 [0-{len(results)-1}] 或邮箱关键字 (按 q 取消): ").strip()
    except (KeyboardInterrupt, EOFError):
        return

    if not choice:
        return
    if choice.lower() in ['q', 'quit', 'exit']:
        return

    if choice.lower() in ['auto', 'best']:
        if best_acc:
            target = best_acc['id']
        else:
            print(f"{C_RED}[-] 未能找到推荐账号。{C_RESET}")
            pause()
            return
    else:
        target = choice

    print(f"\n{C_DIM}[*] 正在执行切换...{C_RESET}")
    success, msg = switch_to_account(target)
    if success:
        print(f"\n{C_BOLD}{C_GREEN}[+] 账号切换成功！当前活动账号已切换为: {msg}{C_RESET}")
        print(f"{C_DIM}凭据已更新至系统凭据管理器，下次运行 `agy` 将自动生效。{C_RESET}")
    else:
        print(f"\n{C_BOLD}{C_RED}[-] 切换失败: {msg}{C_RESET}")
    pause()

def option_auto_switch():
    clear_screen()
    print(f"\n{C_BOLD}{C_CYAN}=== 智能切至最高额度账号 (Auto-Switch) ==={C_RESET}")
    print(f"{C_DIM}正在计算各账号木桶短板有效容量 min(5h, weekly)...{C_RESET}\n")

    results, current_id, best_acc = get_accounts_with_quota()
    if not best_acc:
        print(f"{C_YELLOW}当前没有比活动账号更充沛的其他可用账号，或所有备用账号不可用。{C_RESET}")
        pause()
        return

    print(f"[*] 找到推荐账号: {C_BOLD}{C_MAGENTA}{best_acc['email']}{C_RESET}")
    success, msg = switch_to_account(best_acc['id'])
    if success:
        print(f"\n{C_BOLD}{C_GREEN}[+] 自动切换成功！已成功切至: {msg}{C_RESET}")
    else:
        print(f"\n{C_BOLD}{C_RED}[-] 自动切换失败: {msg}{C_RESET}")
    pause()

def option_save_current_account():
    clear_screen()
    print(f"\n{C_BOLD}{C_CYAN}=== 保存当前活动 agy 账号至账号池 (Save Active Account) ==={C_RESET}")
    print(f"{C_DIM}将从系统凭据管理器抓取当前账号并存入 ~/.antigravity_tools/ 账号池...{C_RESET}\n")

    email, _ = get_current_active_info()
    print(f"识别到当前账号: {C_BOLD}{C_WHITE}{email}{C_RESET}")

    try:
        alias = input("请输入此账号的备注别名 (直接回车跳过): ").strip()
    except (KeyboardInterrupt, EOFError):
        return

    success, msg = save_current_agy_to_pool(alias if alias else None)
    if success:
        print(f"\n{C_BOLD}{C_GREEN}[+] 成功将账号 [{msg}] 快照保存至账号池！{C_RESET}")
    else:
        print(f"\n{C_BOLD}{C_RED}[-] 保存失败: {msg}{C_RESET}")
    pause()

def option_add_account():
    clear_screen()
    print(f"\n{C_BOLD}{C_CYAN}=== 添加新账号到账号池 (Add Account) ==={C_RESET}")
    print("--------------------------------------------------")
    print(" [1] 浏览器一键授权登录 (自动打开 Google 登录并回调)")
    print(" [2] 手动粘贴 Refresh Token 导入")
    print(" [0] 返回主菜单")
    print("--------------------------------------------------")

    try:
        choice = input(">> 请选择添加方式 [0-2]: ").strip()
    except (KeyboardInterrupt, EOFError):
        return

    if choice == "1":
        add_account_via_browser()
    elif choice == "2":
        add_account_via_token()

def add_account_via_token():
    print(f"\n{C_DIM}请输入 Google OAuth Refresh Token (以 1// 开头):{C_RESET}")
    try:
        rf = input("Token: ").strip()
    except (KeyboardInterrupt, EOFError):
        return
    if not rf:
        print("未输入 Token，已取消。")
        pause()
        return

    print(f"\n{C_DIM}[*] 正在验证 Token 并拉取账号信息...{C_RESET}")
    try:
        tok_res = refresh_oauth_token(rf)
        access_tok = tok_res.get('access_token')
        expires_in = tok_res.get('expires_in', 3600)

        req = urllib.request.Request(
            'https://www.googleapis.com/oauth2/v3/userinfo',
            headers={'Authorization': f'Bearer {access_tok}'}
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            uinfo = json.loads(resp.read().decode('utf-8'))

        email = uinfo.get('email')
        name = uinfo.get('name', email.split('@')[0])

        now_ts = int(datetime.now(timezone.utc).timestamp())
        acc_id = str(uuid.uuid4())
        account_obj = {
            "id": acc_id,
            "email": email,
            "name": name,
            "token": {
                "access_token": access_tok,
                "refresh_token": rf,
                "expires_in": expires_in,
                "expiry_timestamp": now_ts + expires_in,
                "token_type": "Bearer",
                "email": email,
                "is_gcp_tos": False
            },
            "created_at": now_ts,
            "last_used": now_ts,
            "disabled": False,
            "proxy_disabled": False
        }
        save_account(account_obj)

        index_data = load_account_index()
        summaries = index_data.get('accounts', [])
        summaries.append({
            "id": acc_id,
            "email": email,
            "name": name,
            "disabled": False,
            "proxy_disabled": False,
            "created_at": now_ts,
            "last_used": now_ts
        })
        index_data['accounts'] = summaries
        save_account_index(index_data)

        print(f"\n{C_BOLD}{C_GREEN}[+] 账号添加成功！邮箱: {email} ({name}){C_RESET}")
    except Exception as e:
        print(f"\n{C_BOLD}{C_RED}[-] 添加失败: {e}{C_RESET}")
    pause()

def add_account_via_browser():
    print(f"\n{C_DIM}[*] 正在启动本地临时授权接收服务 (端口 43210)...{C_RESET}")

    auth_code_holder = {}

    class OAuthHandler(http.server.BaseHTTPRequestHandler):
        def log_message(self, format, *args):
            pass
        def do_GET(self):
            query = urllib.parse.urlparse(self.path).query
            params = urllib.parse.parse_qs(query)
            if 'code' in params:
                auth_code_holder['code'] = params['code'][0]
                self.send_response(200)
                self.send_header('Content-Type', 'text/html; charset=utf-8')
                self.end_headers()
                self.wfile.write("<h1>[+] 授权成功！您可以关闭此页面返回终端了。</h1>".encode('utf-8'))
            else:
                self.send_response(400)
                self.end_headers()

    try:
        server = http.server.HTTPServer(('127.0.0.1', 43210), OAuthHandler)
        server.timeout = 120
    except Exception as e:
        print(f"{C_RED}[-] 启动本地回调服务失败 (端口可能被占用): {e}{C_RESET}")
        pause()
        return

    redirect_uri = "http://127.0.0.1:43210"
    params = {
        'client_id': OAUTH_CLIENT_ID,
        'redirect_uri': redirect_uri,
        'response_type': 'code',
        'scope': 'openid email profile https://www.googleapis.com/auth/cloud-platform',
        'access_type': 'offline',
        'prompt': 'consent'
    }
    auth_url = f"https://accounts.google.com/o/oauth2/v2/auth?{urllib.parse.urlencode(params)}"

    print(f"{C_GREEN}[*] 正在自动打开默认浏览器进行 Google 登录授权...{C_RESET}")
    print(f"{C_DIM}如果浏览器未自动弹出，请手动在浏览器打开以下链接：{C_RESET}\n{auth_url}\n")
    webbrowser.open(auth_url)

    print("等待浏览器回调授权 (超时时间: 120 秒)...")
    while 'code' not in auth_code_holder:
        server.handle_request()

    server.server_close()
    code = auth_code_holder.get('code')
    if not code:
        print(f"{C_RED}[-] 未能获取到授权码。{C_RESET}")
        pause()
        return

    print(f"\n{C_DIM}[*] 正在通过授权码换取 Token...{C_RESET}")
    try:
        token_req_data = {
            'code': code,
            'client_id': OAUTH_CLIENT_ID,
            'client_secret': OAUTH_CLIENT_SECRET,
            'redirect_uri': redirect_uri,
            'grant_type': 'authorization_code'
        }
        req = urllib.request.Request(
            'https://oauth2.googleapis.com/token',
            data=urllib.parse.urlencode(token_req_data).encode('utf-8'),
            headers={'Content-Type': 'application/x-www-form-urlencoded'}
        )
        with urllib.request.urlopen(req, timeout=12) as resp:
            token_res = json.loads(resp.read().decode('utf-8'))

        access_tok = token_res.get('access_token')
        refresh_tok = token_res.get('refresh_token')
        expires_in = token_res.get('expires_in', 3600)

        req = urllib.request.Request(
            'https://www.googleapis.com/oauth2/v3/userinfo',
            headers={'Authorization': f'Bearer {access_tok}'}
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            uinfo = json.loads(resp.read().decode('utf-8'))

        email = uinfo.get('email')
        name = uinfo.get('name', email.split('@')[0])

        now_ts = int(datetime.now(timezone.utc).timestamp())
        acc_id = str(uuid.uuid4())
        account_obj = {
            "id": acc_id,
            "email": email,
            "name": name,
            "token": {
                "access_token": access_tok,
                "refresh_token": refresh_tok,
                "expires_in": expires_in,
                "expiry_timestamp": now_ts + expires_in,
                "token_type": "Bearer",
                "email": email,
                "is_gcp_tos": False
            },
            "created_at": now_ts,
            "last_used": now_ts,
            "disabled": False,
            "proxy_disabled": False
        }
        save_account(account_obj)

        index_data = load_account_index()
        summaries = index_data.get('accounts', [])
        summaries.append({
            "id": acc_id,
            "email": email,
            "name": name,
            "disabled": False,
            "proxy_disabled": False,
            "created_at": now_ts,
            "last_used": now_ts
        })
        index_data['accounts'] = summaries
        save_account_index(index_data)

        print(f"\n{C_BOLD}{C_GREEN}[+] 授权登录成功！账号 [{email}] 已存入账号池！{C_RESET}")
    except Exception as e:
        print(f"\n{C_BOLD}{C_RED}[-] 授权交换失败: {e}{C_RESET}")
    pause()

def option_remove_account():
    clear_screen()
    print(f"\n{C_BOLD}{C_CYAN}=== 从账号池移除账号 (Remove Account) ==={C_RESET}\n")

    accounts, current_id = list_all_accounts()
    if not accounts:
        print(f"{C_YELLOW}账号池为空。{C_RESET}")
        pause()
        return

    for idx, a in enumerate(accounts):
        is_cur = (a['id'] == current_id)
        tag = f"{C_GREEN}[当前*]{C_RESET}" if is_cur else f"{C_DIM}[备用]{C_RESET}"
        print(f"[{idx}] {tag} {a['email']}")

    try:
        choice = input(f"\n>> 请输入要删除的账号序号 [0-{len(accounts)-1}] 或邮箱 (按 q 取消): ").strip()
    except (KeyboardInterrupt, EOFError):
        return

    if not choice or choice.lower() in ['q', 'quit', 'exit']:
        return

    success, msg = remove_account(choice)
    if success:
        print(f"\n{C_BOLD}{C_GREEN}[+] 成功移除账号: {msg}{C_RESET}")
    else:
        print(f"\n{C_BOLD}{C_RED}[-] 移除失败: {msg}{C_RESET}")
    pause()

def option_force_refresh():
    clear_screen()
    print(f"\n{C_DIM}[*] 正在强制刷新当前凭据 Token...{C_RESET}")
    tok = get_valid_token(force_refresh=True)
    if tok:
        print(f"{C_BOLD}{C_GREEN}[+] 成功刷新 OAuth Token 并同步至系统凭据管理器！{C_RESET}")
    else:
        print(f"{C_BOLD}{C_RED}[-] Token 刷新失败。{C_RESET}")
    pause()

def main_menu():
    while True:
        clear_screen()
        email, tier = get_current_active_info()
        print(f"{C_BOLD}{C_CYAN}======================================================================{C_RESET}")
        print(f"{C_BOLD} >>> Antigravity (AGY) 多账号与配额管理中心 v1.2.0 <<<{C_RESET}")
        print(f"{C_BOLD}{C_CYAN}======================================================================{C_RESET}")
        print(f" 当前活动账号: {C_BOLD}{C_WHITE}{email}{C_RESET}  |  套餐类型: {C_YELLOW}{tier}{C_RESET}")
        print(f" 系统本地时间: {C_DIM}{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}{C_RESET}")
        print(f"{'-' * 70}")
        print(f" {C_CYAN}[1]{C_RESET} 查看当前账号额度详情 (Detailed Quota)")
        print(f" {C_CYAN}[2]{C_RESET} 查看所有账号全局大盘 (All Accounts Overview)")
        print(f" {C_CYAN}[3]{C_RESET} 切换当前活动账号 (Switch Account - 交互选择 / 序号 / 邮箱)")
        print(f" {C_CYAN}[4]{C_RESET} 智能切至最高额度账号 (Auto-Switch to Best Quota Account)")
        print(f" {C_CYAN}[5]{C_RESET} 保存当前 agy 账号至账号池 (Save Active agy Account)")
        print(f" {C_CYAN}[6]{C_RESET} 添加新账号到账号池 (Add New Account - 浏览器授权 / Token)")
        print(f" {C_CYAN}[7]{C_RESET} 从账号池移除账号 (Remove Account)")
        print(f" {C_CYAN}[8]{C_RESET} 强制刷新当前账号凭据 (Refresh Token)")
        print(f" {C_RED}[0]{C_RESET} 退出程序 (Exit)")
        print(f"{C_BOLD}{C_CYAN}======================================================================{C_RESET}")

        try:
            choice = input(f">> 请输入选项 {C_BOLD}[0-8]{C_RESET}: ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\n已退出程序。")
            break

        if choice == "1":
            option_view_current_quota()
        elif choice == "2":
            option_view_all_accounts()
        elif choice == "3":
            option_switch_account()
        elif choice == "4":
            option_auto_switch()
        elif choice == "5":
            option_save_current_account()
        elif choice == "6":
            option_add_account()
        elif choice == "7":
            option_remove_account()
        elif choice == "8":
            option_force_refresh()
        elif choice in ["0", "q", "quit", "exit"]:
            print(f"\n{C_GREEN}[+] 感谢使用，再见！{C_RESET}\n")
            break
        else:
            print(f"{C_RED}无效的选项，请重新输入。{C_RESET}")
            time.sleep(0.8)

if __name__ == "__main__":
    main_menu()
