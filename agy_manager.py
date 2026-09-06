#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Antigravity Core Account & Quota Manager
Pure Python standard library implementation compatible with Antigravity-Manager storage.
"""

import os
import sys
import json
import uuid
import ctypes
from ctypes import wintypes
import urllib.request
import urllib.parse
import urllib.error
from datetime import datetime, timezone

# Ensure UTF-8 output
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

# Constants
OAUTH_CLIENT_ID = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
OAUTH_CLIENT_SECRET = "GOCSPX-" + "K58FWR486LdLJ1mLB8sXC4z6qDAf"
USER_AGENT = "antigravity/1.1.24"
API_BASE = "https://daily-cloudcode-pa.googleapis.com/v1internal"

QUOTA_SUMMARY_ENDPOINTS = [
    "https://daily-cloudcode-pa.sandbox.googleapis.com/v1internal:retrieveUserQuotaSummary",
    "https://daily-cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
    "https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuotaSummary",
]

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

import unicodedata
import re
ANSI_RE = re.compile(r'\033\[[0-9;]*m')

def strip_ansi(text):
    return ANSI_RE.sub('', text)

def visual_width(text):
    plain = strip_ansi(text)
    w = 0
    for ch in plain:
        if unicodedata.east_asian_width(ch) in ('F', 'W'):
            w += 2
        else:
            w += 1
    return w

def pad_visual(text, target_width, align='left'):
    w = visual_width(text)
    pad = max(0, target_width - w)
    if align == 'right':
        return ' ' * pad + text
    return text + ' ' * pad

# Windows Credential Manager
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

def get_data_dir():
    custom = os.environ.get("ABV_DATA_DIR")
    if custom and custom.strip():
        d = custom.strip()
        os.makedirs(d, exist_ok=True)
        return d
    home = os.path.expanduser("~")
    # Centralized official directory: ~/.gemini
    primary = os.path.join(home, ".gemini")
    os.makedirs(primary, exist_ok=True)
    return primary

def get_accounts_dir():
    d = os.path.join(get_data_dir(), "accounts")
    os.makedirs(d, exist_ok=True)
    return d

def get_index_path():
    return os.path.join(get_data_dir(), "accounts.json")

def read_keyring(target="gemini:antigravity"):
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

def write_keyring(data, target="gemini:antigravity"):
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
    with urllib.request.urlopen(req, timeout=12) as resp:
        return json.loads(resp.read().decode('utf-8'))

def get_backup_dir():
    home = os.path.expanduser("~")
    d = os.path.join(home, ".gemini", "accounts_backup")
    os.makedirs(d, exist_ok=True)
    return d

def load_account_index():
    path = get_index_path()
    if not os.path.exists(path) or os.path.getsize(path) == 0:
        # 1. Try migrating from legacy ~/.antigravity_tools if it exists
        legacy_dir = os.path.join(os.path.expanduser("~"), ".antigravity_tools")
        legacy_index = os.path.join(legacy_dir, "accounts.json")
        if os.path.exists(legacy_index) and os.path.getsize(legacy_index) > 0:
            try:
                import shutil
                shutil.copy2(legacy_index, path)
                leg_accs = os.path.join(legacy_dir, "accounts")
                if os.path.exists(leg_accs):
                    shutil.copytree(leg_accs, get_accounts_dir(), dirs_exist_ok=True)
            except Exception:
                pass

        # 2. Try auto-recovery from backup if still missing
        backup_index = os.path.join(get_backup_dir(), "accounts.json")
        if (not os.path.exists(path) or os.path.getsize(path) == 0) and os.path.exists(backup_index):
            try:
                import shutil
                shutil.copy2(backup_index, path)
                b_accs = os.path.join(get_backup_dir(), "accounts")
                if os.path.exists(b_accs):
                    shutil.copytree(b_accs, get_accounts_dir(), dirs_exist_ok=True)
            except Exception:
                pass

    if not os.path.exists(path):
        return {"version": "2.0", "accounts": [], "current_account_id": None}
    try:
        with open(path, 'r', encoding='utf-8') as f:
            return json.load(f)
    except Exception:
        return {"version": "2.0", "accounts": [], "current_account_id": None}

def save_account_index(data):
    path = get_index_path()
    temp_path = f"{path}.tmp.{uuid.uuid4().hex[:8]}"
    with open(temp_path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    os.replace(temp_path, path)
    # Sync to backup
    try:
        backup_path = os.path.join(get_backup_dir(), "accounts.json")
        with open(backup_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
    except Exception:
        pass

def load_account(account_id):
    path = os.path.join(get_accounts_dir(), f"{account_id}.json")
    if not os.path.exists(path):
        # check backup
        backup_path = os.path.join(get_backup_dir(), "accounts", f"{account_id}.json")
        if os.path.exists(backup_path):
            try:
                import shutil
                shutil.copy2(backup_path, path)
            except Exception:
                pass
    if not os.path.exists(path):
        return None
    try:
        with open(path, 'r', encoding='utf-8') as f:
            return json.load(f)
    except Exception:
        return None

def save_account(account):
    acc_id = account['id']
    path = os.path.join(get_accounts_dir(), f"{acc_id}.json")
    temp_path = f"{path}.tmp.{uuid.uuid4().hex[:8]}"
    with open(temp_path, 'w', encoding='utf-8') as f:
        json.dump(account, f, indent=2, ensure_ascii=False)
    os.replace(temp_path, path)
    # Sync to backup
    try:
        b_accs_dir = os.path.join(get_backup_dir(), "accounts")
        os.makedirs(b_accs_dir, exist_ok=True)
        b_path = os.path.join(b_accs_dir, f"{acc_id}.json")
        with open(b_path, 'w', encoding='utf-8') as f:
            json.dump(account, f, indent=2, ensure_ascii=False)
    except Exception:
        pass

def list_all_accounts():
    index_data = load_account_index()
    accounts = []
    for summary in index_data.get('accounts', []):
        acc = load_account(summary['id'])
        if acc:
            accounts.append(acc)
    return accounts, index_data.get('current_account_id')

def ensure_valid_token(account):
    """Refreshes account token if expired, saving updated token to disk."""
    tok = account.get('token', {})
    access_tok = tok.get('access_token')
    refresh_tok = tok.get('refresh_token')
    expiry_ts = tok.get('expiry_timestamp')

    now_ts = int(datetime.now(timezone.utc).timestamp())
    need_refresh = False

    if not access_tok:
        need_refresh = True
    elif expiry_ts and (expiry_ts - now_ts) < 60:
        need_refresh = True

    if need_refresh and refresh_tok:
        try:
            res = refresh_oauth_token(refresh_tok)
            access_tok = res.get('access_token')
            expires_in = res.get('expires_in', 3600)
            account['token']['access_token'] = access_tok
            account['token']['expires_in'] = expires_in
            account['token']['expiry_timestamp'] = now_ts + expires_in
            save_account(account)
        except Exception as e:
            return None, str(e)

    return access_tok, None

def fetch_account_quota(account):
    """Fetches quota summary for an account with token auto-refresh and 403 detection."""
    access_tok, err = ensure_valid_token(account)
    if not access_tok:
        return None, f"Token refresh failed: {err}"

    last_err = None
    for ep in QUOTA_SUMMARY_ENDPOINTS:
        req = urllib.request.Request(
            ep,
            data=b'{}',
            headers={
                'Content-Type': 'application/json',
                'Authorization': f'Bearer {access_tok}',
                'User-Agent': USER_AGENT
            },
            method='POST'
        )
        try:
            with urllib.request.urlopen(req, timeout=12) as resp:
                return json.loads(resp.read().decode('utf-8')), None
        except urllib.error.HTTPError as e:
            last_err = e
            if e.code == 401:
                # Force refresh once
                refresh_tok = account.get('token', {}).get('refresh_token')
                if refresh_tok:
                    try:
                        res = refresh_oauth_token(refresh_tok)
                        access_tok = res.get('access_token')
                        account['token']['access_token'] = access_tok
                        save_account(account)
                        req.headers['Authorization'] = f'Bearer {access_tok}'
                        with urllib.request.urlopen(req, timeout=12) as retry_resp:
                            return json.loads(retry_resp.read().decode('utf-8')), None
                    except Exception:
                        pass
                return None, "401 Unauthorized"
            elif e.code == 403:
                return None, "403 License Required / Forbidden"
            elif e.code == 429:
                continue
        except Exception as e:
            last_err = e
            continue

    return None, str(last_err)

def parse_quota_buckets(summary):
    """Extracts remaining fractions and reset times for Gemini 5h, Gemini Weekly, Claude 5h, Claude Weekly."""
    gemini_5h, gemini_w, claude_5h, claude_w = None, None, None, None
    gemini_5h_reset, gemini_w_reset = None, None
    claude_5h_reset, claude_w_reset = None, None
    if not summary:
        return None

    for g in summary.get('groups', []):
        gname = g.get('displayName', '').lower()
        is_claude = ('claude' in gname or 'gpt' in gname)
        for b in g.get('buckets', []):
            win = b.get('window', '')
            frac = b.get('remainingFraction', 0.0)
            rt = b.get('resetTime')
            if is_claude:
                if win == '5h':
                    claude_5h = frac
                    claude_5h_reset = rt
                elif win == 'weekly':
                    claude_w = frac
                    claude_w_reset = rt
            else:
                if win == '5h':
                    gemini_5h = frac
                    gemini_5h_reset = rt
                elif win == 'weekly':
                    gemini_w = frac
                    gemini_w_reset = rt

    return {
        'gemini_5h': gemini_5h,
        'gemini_weekly': gemini_w,
        'gemini_5h_reset': gemini_5h_reset,
        'gemini_weekly_reset': gemini_w_reset,
        'claude_5h': claude_5h,
        'claude_weekly': claude_w,
        'claude_5h_reset': claude_5h_reset,
        'claude_weekly_reset': claude_w_reset
    }

from datetime import datetime, timezone

def format_compact_countdown(target_iso, frac=1.0, with_suffix=False):
    if frac is not None and frac >= 0.999:
        return f"{C_GREEN}满额{C_RESET}"
    if not target_iso:
        return f"{C_DIM}--{C_RESET}"
    try:
        clean_iso = re.sub(r'\.\d+', '', target_iso).replace('Z', '+00:00')
        target_dt = datetime.fromisoformat(clean_iso)
        now_dt = datetime.now(timezone.utc)
        diff = target_dt - now_dt
        total_sec = int(diff.total_seconds())
        if total_sec <= 0:
            return f"{C_GREEN}已就绪{C_RESET}"
        hours = total_sec // 3600
        mins = (total_sec % 3600) // 60
        suf = "后" if with_suffix else ""
        if hours >= 24:
            days = hours // 24
            rem_h = hours % 24
            if rem_h > 0:
                return f"{C_YELLOW}{days}天{rem_h:02d}h{suf}{C_RESET}"
            else:
                return f"{C_YELLOW}{days}天{suf}{C_RESET}"
        elif hours > 0:
            return f"{C_YELLOW}{hours}h{mins:02d}m{suf}{C_RESET}"
        else:
            return f"{C_MAGENTA}{mins}分{suf}{C_RESET}"
    except Exception:
        return f"{C_DIM}--{C_RESET}"

def format_countdown_pair(cd_5h, cd_w):
    s_5h = cd_5h or f"{C_DIM}--{C_RESET}"
    s_w = cd_w or f"{C_DIM}--{C_RESET}"
    return f"{pad_visual(s_5h, 6, align='right')} / {pad_visual(s_w, 6, align='right')}"

def calculate_effective_quota(parsed):
    """
    Evaluates account redundancy using bottleneck principle: min(5h, weekly).
    Combines Claude and Gemini capacity.
    """
    if not parsed:
        return -1.0
    c_5h = parsed.get('claude_5h')
    c_w = parsed.get('claude_weekly')
    g_5h = parsed.get('gemini_5h')
    g_w = parsed.get('gemini_weekly')

    claude_eff = 1.0
    if c_5h is not None and c_w is not None:
        claude_eff = min(c_5h, c_w)
    elif c_w is not None:
        claude_eff = c_w

    gemini_eff = 1.0
    if g_5h is not None and g_w is not None:
        gemini_eff = min(g_5h, g_w)
    elif g_w is not None:
        gemini_eff = g_w

    # Claude quota is weighted higher due to higher scarcity
    return (claude_eff * 0.6) + (gemini_eff * 0.4)

def switch_to_account(account_id_or_keyword):
    """
    Core one-click account switch implementation:
    1. Locates account
    2. Validates/refreshes token
    3. Writes to Windows Credential Manager ('gemini:antigravity')
    4. Syncs ~/.gemini/google_accounts.json and oauth_creds.json
    5. Updates ~/.antigravity_tools/accounts.json current_account_id
    """
    accounts, current_id = list_all_accounts()
    target_account = None

    # Check numeric index
    if str(account_id_or_keyword).isdigit():
        idx = int(account_id_or_keyword)
        if 0 <= idx < len(accounts):
            target_account = accounts[idx]

    # Check UUID or email keyword
    if not target_account:
        kw = str(account_id_or_keyword).strip().lower()
        for a in accounts:
            if a['id'].lower() == kw or kw in a['email'].lower() or (a.get('name') and kw in a['name'].lower()):
                target_account = a
                break

    if not target_account:
        return False, f"未找到匹配的账号: '{account_id_or_keyword}'"

    email = target_account['email']
    acc_id = target_account['id']

    # 1. Ensure token is fresh
    access_tok, err = ensure_valid_token(target_account)
    if not access_tok:
        return False, f"刷新目标账号 Token 失败: {err}"

    tok_data = target_account.get('token', {})
    refresh_tok = tok_data.get('refresh_token')
    expiry_ts = tok_data.get('expiry_timestamp')
    if expiry_ts:
        dt = datetime.fromtimestamp(expiry_ts, tz=timezone.utc)
        expiry_iso = dt.isoformat()
    else:
        expiry_iso = datetime.now(timezone.utc).isoformat()

    # 2. Write to Windows Credential Manager
    keyring_payload = {
        "token": {
            "access_token": access_tok,
            "token_type": "Bearer",
            "refresh_token": refresh_tok,
            "expiry": expiry_iso
        },
        "auth_method": "consumer"
    }
    if not write_keyring(keyring_payload):
        return False, "写入 Windows 凭据管理器 (gemini:antigravity) 失败！"

    # 3. Sync ~/.gemini/google_accounts.json
    home = os.path.expanduser("~")
    ga_file = os.path.join(home, ".gemini", "google_accounts.json")
    try:
        old_accounts = []
        if os.path.exists(ga_file):
            with open(ga_file, 'r', encoding='utf-8') as f:
                old_data = json.load(f)
                prev_active = old_data.get('active')
                old_accounts = old_data.get('old', [])
                if prev_active and prev_active != email and prev_active not in old_accounts:
                    old_accounts.append(prev_active)
        with open(ga_file, 'w', encoding='utf-8') as f:
            json.dump({"active": email, "old": old_accounts}, f, indent=2)
    except Exception as e:
        print(f"{C_YELLOW}[!] 警告: 更新 google_accounts.json 失败: {e}{C_RESET}", file=sys.stderr)

    # 4. Sync ~/.gemini/oauth_creds.json
    oc_file = os.path.join(home, ".gemini", "oauth_creds.json")
    try:
        oc_data = {
            "access_token": access_tok,
            "refresh_token": refresh_tok,
            "token_type": "Bearer",
            "expiry_date": (expiry_ts * 1000) if expiry_ts else int(datetime.now().timestamp() * 1000) + 3600000
        }
        with open(oc_file, 'w', encoding='utf-8') as f:
            json.dump(oc_data, f, indent=2)
    except Exception:
        pass

    # 5. Update ~/.antigravity_tools/accounts.json
    index_data = load_account_index()
    index_data['current_account_id'] = acc_id
    index_data['current_target_ide'] = "agy"
    for s in index_data.get('accounts', []):
        if s['id'] == acc_id:
            s['last_used'] = int(datetime.now().timestamp())
    save_account_index(index_data)

    target_account['last_used'] = int(datetime.now().timestamp())
    save_account(target_account)

    return True, email

def save_current_agy_to_pool(name=None):
    """Snapshots current active credentials from Windows Keyring into the ~/.antigravity_tools pool."""
    keyring_data = read_keyring()
    if not keyring_data or 'token' not in keyring_data:
        return False, "当前未在系统凭据中找到任何有效凭据。"

    tok = keyring_data['token']
    access_tok = tok.get('access_token')
    refresh_tok = tok.get('refresh_token')
    if not access_tok or not refresh_tok:
        return False, "凭据不完整（缺少 access_token 或 refresh_token）。"

    # Get email
    home = os.path.expanduser("~")
    ga_file = os.path.join(home, ".gemini", "google_accounts.json")
    email = None
    if os.path.exists(ga_file):
        try:
            with open(ga_file, 'r', encoding='utf-8') as f:
                email = json.load(f).get('active')
        except Exception:
            pass

    if not email:
        return False, "未能识别当前活动账号的邮箱。"

    accounts, _ = list_all_accounts()
    existing = None
    for a in accounts:
        if a['email'].lower() == email.lower():
            existing = a
            break

    now_ts = int(datetime.now(timezone.utc).timestamp())
    acc_id = existing['id'] if existing else str(uuid.uuid4())

    account_obj = existing if existing else {
        "id": acc_id,
        "email": email,
        "name": name or email.split('@')[0],
        "token": {},
        "created_at": now_ts,
        "last_used": now_ts,
        "disabled": False,
        "proxy_disabled": False
    }

    if name:
        account_obj['name'] = name

    account_obj['token'] = {
        "access_token": access_tok,
        "refresh_token": refresh_tok,
        "expires_in": 3600,
        "expiry_timestamp": now_ts + 3600,
        "token_type": "Bearer",
        "email": email,
        "is_gcp_tos": False
    }
    account_obj['last_used'] = now_ts
    save_account(account_obj)

    # Update index
    index_data = load_account_index()
    index_data['current_account_id'] = acc_id
    summaries = index_data.get('accounts', [])
    found = False
    for s in summaries:
        if s['id'] == acc_id:
            s['last_used'] = now_ts
            if name: s['name'] = name
            found = True
            break
    if not found:
        summaries.append({
            "id": acc_id,
            "email": email,
            "name": name or email.split('@')[0],
            "disabled": False,
            "proxy_disabled": False,
            "created_at": now_ts,
            "last_used": now_ts
        })
    index_data['accounts'] = summaries
    save_account_index(index_data)

    return True, email

def remove_account(account_id_or_keyword):
    """Removes an account from ~/.antigravity_tools."""
    accounts, current_id = list_all_accounts()
    target_account = None

    if str(account_id_or_keyword).isdigit():
        idx = int(account_id_or_keyword)
        if 0 <= idx < len(accounts):
            target_account = accounts[idx]

    if not target_account:
        kw = str(account_id_or_keyword).strip().lower()
        for a in accounts:
            if a['id'].lower() == kw or kw in a['email'].lower():
                target_account = a
                break

    if not target_account:
        return False, f"未找到要删除的账号: '{account_id_or_keyword}'"

    acc_id = target_account['id']
    email = target_account['email']

    # Delete json file
    acc_file = os.path.join(get_accounts_dir(), f"{acc_id}.json")
    if os.path.exists(acc_file):
        try:
            os.remove(acc_file)
        except Exception as e:
            return False, f"删除账号文件失败: {e}"

    # Remove from index
    index_data = load_account_index()
    index_data['accounts'] = [s for s in index_data.get('accounts', []) if s['id'] != acc_id]
    if index_data.get('current_account_id') == acc_id:
        index_data['current_account_id'] = None
    save_account_index(index_data)

    return True, email
