#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AGY Usage & Quota CLI (Antigravity 额度/使用量查看工具)
Directly fetches and displays remaining quota for Antigravity (Gemini & Claude/GPT models).
Zero external dependencies, pure Python standard library.
"""

import os
import sys
import json
import time
import ctypes
from ctypes import wintypes
import urllib.request
import urllib.parse
import urllib.error
import subprocess
import argparse
from datetime import datetime, timezone

# Ensure stdout and stderr use utf-8
if hasattr(sys.stdout, 'reconfigure'):
    try:
        sys.stdout.reconfigure(encoding='utf-8')
    except Exception:
        pass
if hasattr(sys.stderr, 'reconfigure'):
    try:
        sys.stderr.reconfigure(encoding='utf-8')
    except Exception:
        pass

# Windows Console ANSI Enable & UTF-8 Codepage
if sys.platform == "win32":
    try:
        kernel32 = ctypes.windll.kernel32
        kernel32.SetConsoleOutputCP(65001)
        kernel32.SetConsoleCP(65001)
        hStdOut = kernel32.GetStdHandle(-11)
        mode = wintypes.DWORD()
        kernel32.GetConsoleMode(hStdOut, ctypes.byref(mode))
        kernel32.SetConsoleMode(hStdOut, mode.value | 0x0004)
    except Exception:
        pass

# Color codes
C_RESET = "\033[0m"
C_BOLD = "\033[1m"
C_DIM = "\033[2m"
C_GREEN = "\033[92m"
C_YELLOW = "\033[93m"
C_RED = "\033[91m"
C_CYAN = "\033[96m"
C_BLUE = "\033[94m"
C_MAGENTA = "\033[95m"
C_WHITE = "\033[97m"

OAUTH_CLIENT_ID = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
OAUTH_CLIENT_SECRET = "GOCSPX-" + "K58FWR486LdLJ1mLB8sXC4z6qDAf"
USER_AGENT = "antigravity/1.1.24"
API_BASE = "https://daily-cloudcode-pa.googleapis.com/v1internal"

# Windows Credential Manager structures
advapi32 = ctypes.windll.advapi32

class CREDENTIAL(ctypes.Structure):
    _fields_ = [
        ('Flags', wintypes.DWORD),
        ('Type', wintypes.DWORD),
        ('TargetName', wintypes.LPWSTR),
        ('Comment', wintypes.LPWSTR),
        ('LastWritten', wintypes.FILETIME),
        ('CredentialBlobSize', wintypes.DWORD),
        ('CredentialBlob', ctypes.c_char_p),
        ('Persist', wintypes.DWORD),
        ('AttributeCount', wintypes.DWORD),
        ('Attributes', ctypes.c_void_p),
        ('TargetAlias', wintypes.LPWSTR),
        ('UserName', wintypes.LPWSTR),
    ]

def read_keyring_credential(target="gemini:antigravity"):
    cred_ptr = ctypes.POINTER(CREDENTIAL)()
    if not advapi32.CredReadW(target, 1, 0, ctypes.byref(cred_ptr)):
        return None
    try:
        cred = cred_ptr.contents
        raw = ctypes.string_at(cred.CredentialBlob, cred.CredentialBlobSize)
        return json.loads(raw.decode('utf-8'))
    except Exception:
        return None
    finally:
        advapi32.CredFree(cred_ptr)

def write_keyring_credential(data, target="gemini:antigravity"):
    try:
        raw = json.dumps(data).encode('utf-8')
        cred = CREDENTIAL()
        cred.Flags = 0
        cred.Type = 1  # CRED_TYPE_GENERIC
        cred.TargetName = target
        cred.Comment = "Antigravity CLI credentials"
        cred.CredentialBlobSize = len(raw)
        cred.CredentialBlob = raw
        cred.Persist = 2  # CRED_PERSIST_LOCAL_MACHINE
        cred.AttributeCount = 0
        cred.Attributes = None
        cred.TargetAlias = None
        cred.UserName = "antigravity"
        return bool(advapi32.CredWriteW(ctypes.byref(cred), 0))
    except Exception:
        return False

def refresh_oauth_token(refresh_token):
    body = {
        'client_id': OAUTH_CLIENT_ID,
        'client_secret': OAUTH_CLIENT_SECRET,
        'grant_type': 'refresh_token',
        'refresh_token': refresh_token,
    }
    req = urllib.request.Request(
        'https://oauth2.googleapis.com/token',
        data=urllib.parse.urlencode(body).encode('utf-8'),
        headers={'Content-Type': 'application/x-www-form-urlencoded'}
    )
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read().decode('utf-8'))

def trigger_cli_refresh():
    try:
        subprocess.run(["agy", "models"], capture_output=True, text=True, timeout=12)
        return True
    except Exception:
        return False

def get_valid_token(force_refresh=False):
    cred_data = read_keyring_credential()
    token = None
    refresh_tok = None
    expiry_str = None

    if cred_data:
        tok_obj = cred_data.get('token', {})
        token = tok_obj.get('access_token')
        refresh_tok = tok_obj.get('refresh_token')
        expiry_str = tok_obj.get('expiry')

    # Fallback to ~/.gemini/oauth_creds.json
    if not token or not refresh_tok:
        home_dir = os.path.expanduser("~")
        creds_file = os.path.join(home_dir, ".gemini", "oauth_creds.json")
        if os.path.exists(creds_file):
            try:
                with open(creds_file, 'r', encoding='utf-8') as f:
                    file_creds = json.load(f)
                    token = file_creds.get('access_token')
                    refresh_tok = file_creds.get('refresh_token')
            except Exception:
                pass

    # Check if expired
    is_expired = force_refresh
    if expiry_str and not is_expired:
        try:
            dt = datetime.fromisoformat(expiry_str)
            now = datetime.now(timezone.utc)
            if (dt - now).total_seconds() < 45:
                is_expired = True
        except Exception:
            pass

    if (is_expired or not token) and refresh_tok:
        try:
            new_tok = refresh_oauth_token(refresh_tok)
            token = new_tok.get('access_token')
            now_dt = datetime.now(timezone.utc)
            # Update keyring
            if cred_data and isinstance(cred_data.get('token'), dict):
                cred_data['token']['access_token'] = token
                cred_data['token']['expiry'] = now_dt.isoformat()
                write_keyring_credential(cred_data)
        except Exception:
            # Fallback to agy models invocation
            trigger_cli_refresh()
            refreshed_data = read_keyring_credential()
            if refreshed_data:
                token = refreshed_data.get('token', {}).get('access_token')

    return token

def call_api(endpoint, token, body=None):
    if body is None:
        body = {}
    url = f"{API_BASE}:{endpoint}"
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode('utf-8'),
        headers={
            'Content-Type': 'application/json',
            'Authorization': f'Bearer {token}',
            'User-Agent': USER_AGENT
        },
        method='POST'
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode('utf-8'))
    except urllib.error.HTTPError as e:
        if e.code == 401:
            new_token = get_valid_token(force_refresh=True)
            if new_token and new_token != token:
                req.headers['Authorization'] = f'Bearer {new_token}'
                with urllib.request.urlopen(req, timeout=15) as retry_resp:
                    return json.loads(retry_resp.read().decode('utf-8'))
        raise

def format_relative_time(iso_str):
    if not iso_str:
        return "N/A", "N/A"
    try:
        dt = datetime.fromisoformat(iso_str.replace('Z', '+00:00'))
        now = datetime.now(timezone.utc)
        diff = dt - now
        total_seconds = int(diff.total_seconds())

        local_time_str = dt.astimezone().strftime("%m-%d %H:%M")

        if total_seconds <= 0:
            return "已重置", local_time_str

        days = total_seconds // 86400
        hours = (total_seconds % 86400) // 3600
        mins = (total_seconds % 3600) // 60

        if days > 0:
            return f"{days}天 {hours}小时后", local_time_str
        elif hours > 0:
            return f"{hours}小时 {mins}分后", local_time_str
        else:
            return f"{mins}分钟后", local_time_str
    except Exception:
        return iso_str, iso_str

def make_bar(fraction, width=22):
    fraction = max(0.0, min(1.0, fraction))
    filled = int(round(fraction * width))
    empty = width - filled

    if fraction >= 0.50:
        color = C_GREEN
    elif fraction >= 0.20:
        color = C_YELLOW
    else:
        color = C_RED

    bar = f"{color}{'█' * filled}{C_DIM}{'░' * empty}{C_RESET}"
    pct = f"{fraction * 100:5.1f}%"
    return f"[{bar}] {color}{pct}{C_RESET}"

def render_display(summary, user_quota=None, code_assist=None):
    account_email = "未知账户"
    tier_name = "Antigravity"

    home_dir = os.path.expanduser("~")
    ga_file = os.path.join(home_dir, ".gemini", "google_accounts.json")
    if os.path.exists(ga_file):
        try:
            with open(ga_file, 'r', encoding='utf-8') as f:
                account_email = json.load(f).get('active', account_email)
        except Exception:
            pass

    if code_assist and 'currentTier' in code_assist:
        tier_name = code_assist['currentTier'].get('name', tier_name)

    lines = []
    lines.append(f"{C_BOLD}{C_CYAN}================================================================{C_RESET}")
    lines.append(f"{C_BOLD} >>> Antigravity CLI Usage & Quota Monitor (AGY 额度实时看板) <<<{C_RESET}")
    lines.append(f"{C_BOLD}{C_CYAN}================================================================{C_RESET}")
    lines.append(f" 账号: {C_WHITE}{account_email}{C_RESET}  |  套餐: {C_YELLOW}{tier_name}{C_RESET}")
    lines.append(f" 时间: {C_DIM}{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}{C_RESET}")
    lines.append("")

    groups = summary.get('groups', [])
    for g in groups:
        title = g.get('displayName', 'Group')
        desc = g.get('description', '')
        buckets = g.get('buckets', [])

        if "gemini" in title.lower():
            tag = "[Gemini Models]"
            color_hdr = C_BLUE
        else:
            tag = "[Claude / GPT Models]"
            color_hdr = C_MAGENTA

        lines.append(f"{C_BOLD}{color_hdr}{tag} {title}{C_RESET}  {C_DIM}({desc}){C_RESET}")
        lines.append(f"  {'-' * 60}")

        def bucket_sort_key(item):
            w = item.get('window', '')
            if w == '5h': return 0
            if w == 'weekly': return 1
            return 2

        for b in sorted(buckets, key=bucket_sort_key):
            b_name = b.get('displayName', b.get('bucketId', 'Limit'))
            frac = b.get('remainingFraction', 0.0)
            reset_time = b.get('resetTime')
            rel_time, local_time = format_relative_time(reset_time)

            window = b.get('window', '')
            if window == '5h':
                label = "  [5小时限额]"
            elif window == 'weekly':
                label = "  [每周限额  ]"
            else:
                label = f"  [{b_name[:8]}]"

            bar_str = make_bar(frac, width=22)
            reset_hint = f"{C_DIM}(刷新: {rel_time} | 本地: {local_time}){C_RESET}" if reset_time else ""
            lines.append(f"{label} : {bar_str}  {reset_hint}")
        lines.append("")

    if user_quota and 'buckets' in user_quota:
        lines.append(f"{C_BOLD}{C_WHITE}[模型明细统计] (Individual Model Quotas):{C_RESET}")
        lines.append(f"  {'-' * 60}")

        model_buckets = user_quota.get('buckets', [])
        seen_models = set()
        for mb in model_buckets:
            mid = mb.get('modelId')
            if not mid or mid in seen_models:
                continue
            seen_models.add(mid)

            frac = mb.get('remainingFraction', 1.0)
            reset_time = mb.get('resetTime')
            rel_time, _ = format_relative_time(reset_time)
            reset_str = f"重置: {rel_time}" if reset_time else "全满"

            bar_str = make_bar(frac, width=14)
            lines.append(f"  - {mid:<28} {bar_str}  {C_DIM}{reset_str}{C_RESET}")
        lines.append("")

    lines.append(f"{C_DIM}[提示]: 限额按模型消耗度量。Gemini 与 Claude 各组内共享每周与5小时窗口。{C_RESET}")
    return "\n".join(lines)

def main():
    parser = argparse.ArgumentParser(
        description="Antigravity (AGY) Usage & Quota CLI - 一键查看 AGY 剩余额度与重置时间",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  agy-usage              查看当前剩余额度与重置倒计时
  agy-usage -v           查看包括各具体模型细则的详细额度
  agy-usage --json       以 JSON 格式输出额度信息（供自动化脚本调用）
  agy-usage -w 10        每 10 秒自动刷新一次
        """
    )
    parser.add_argument("-a", "--all", dest="all_accounts", action="store_true", help="查看所有已保存账号的全局额度对比看板")
    parser.add_argument("-v", "--verbose", "--details", dest="details", action="store_true", help="显示各模型的单独额度细则")
    parser.add_argument("--json", action="store_true", help="输出原始 JSON 格式数据")
    parser.add_argument("-w", "--watch", type=int, nargs="?", const=5, help="保持监听模式，每隔 N 秒自动刷新一次 (默认 5s)")
    parser.add_argument("--refresh", action="store_true", help="强制立即刷新认证 Token")
    parser.add_argument("--raw", action="store_true", help="打印原始 API 返回数据")

    args = parser.parse_args()

    if args.all_accounts:
        from agy_account import list_accounts_dashboard
        list_accounts_dashboard()
        return

    token = get_valid_token(force_refresh=args.refresh)
    if not token:
        print(f"{C_RED}[-] 错误: 未能获取到 Antigravity 认证凭证。请先在终端运行一次 `agy` 完成登录。{C_RESET}", file=sys.stderr)
        sys.exit(1)

    def fetch_all():
        summary = call_api('retrieveUserQuotaSummary', token)
        user_quota = None
        code_assist = None
        try:
            code_assist = call_api('loadCodeAssist', token)
        except Exception:
            pass
        if args.details or args.json or args.raw:
            try:
                user_quota = call_api('retrieveUserQuota', token)
            except Exception:
                pass
        return summary, user_quota, code_assist

    if args.json:
        summary, user_quota, code_assist = fetch_all()
        out = {
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'summary': summary,
            'userQuota': user_quota,
            'codeAssist': code_assist
        }
        print(json.dumps(out, indent=2, ensure_ascii=False))
        return

    if args.raw:
        summary, user_quota, code_assist = fetch_all()
        print("=== QUOTA SUMMARY ===")
        print(json.dumps(summary, indent=2, ensure_ascii=False))
        if user_quota:
            print("\n=== USER QUOTA ===")
            print(json.dumps(user_quota, indent=2, ensure_ascii=False))
        return

    if args.watch:
        interval = max(1, args.watch)
        try:
            while True:
                os.system('cls' if os.name == 'nt' else 'clear')
                summary, user_quota, code_assist = fetch_all()
                print(render_display(summary, user_quota if args.details else None, code_assist))
                print(f"\n{C_DIM}[Watch 模式: 每 {interval} 秒刷新一次，按 Ctrl+C 退出]{C_RESET}")
                time.sleep(interval)
        except KeyboardInterrupt:
            print("\n已退出监控。")
            return

    # Default single run
    try:
        summary, user_quota, code_assist = fetch_all()
        print(render_display(summary, user_quota if args.details else None, code_assist))
    except Exception as e:
        print(f"{C_RED}[-] 查询额度失败: {e}{C_RESET}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
