use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use unicode_width::UnicodeWidthStr;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const OAUTH_CLIENT_ID: &str = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const OAUTH_CLIENT_SECRET: &str = concat!("GOCSPX-", "K58FWR486LdLJ1mLB8sXC4z6qDAf");
const USER_AGENT: &str = "antigravity/1.1.24";
const API_BASE: &str = "https://daily-cloudcode-pa.googleapis.com/v1internal";

// ANSI Colors
const C_RESET: &str = "\x1B[0m";
const C_BOLD: &str = "\x1B[1m";
const C_DIM: &str = "\x1B[2m";
const C_GREEN: &str = "\x1B[92m";
const C_YELLOW: &str = "\x1B[93m";
const C_RED: &str = "\x1B[91m";
const C_CYAN: &str = "\x1B[96m";
const C_BLUE: &str = "\x1B[94m";
const C_MAGENTA: &str = "\x1B[95m";
const C_WHITE: &str = "\x1B[97m";

// Windows Credential Manager API
#[repr(C)]
struct CREDENTIALW {
    flags: u32,
    cred_type: u32,
    target_name: *mut u16,
    comment: *mut u16,
    last_written: [u32; 2],
    credential_blob_size: u32,
    credential_blob: *mut u8,
    persist: u32,
    attribute_count: u32,
    attributes: *mut std::ffi::c_void,
    target_alias: *mut u16,
    user_name: *mut u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct KEY_EVENT_RECORD {
    b_key_down: i32,
    w_repeat_count: u16,
    w_virtual_key_code: u16,
    w_virtual_scan_code: u16,
    u_char: u16,
    dw_control_key_state: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct INPUT_RECORD {
    event_type: u16,
    _pad: u16,
    key_event: KEY_EVENT_RECORD,
}

#[repr(C)]
struct PROCESSENTRY32W {
    dw_size: u32,
    cnt_usage: u32,
    th32_process_id: u32,
    th32_default_heap_id: usize,
    th32_module_id: u32,
    cnt_threads: u32,
    th32_parent_process_id: u32,
    pc_pri_class_base: i32,
    dw_flags: u32,
    sz_exe_file: [u16; 260],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct COORD {
    x: i16,
    y: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SMALL_RECT {
    left: i16,
    top: i16,
    right: i16,
    bottom: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CONSOLE_SCREEN_BUFFER_INFO {
    dw_size: COORD,
    dw_cursor_position: COORD,
    w_attributes: u16,
    sr_window: SMALL_RECT,
    dw_maximum_window_size: COORD,
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn CredReadW(target: *const u16, cred_type: u32, flags: u32, cred: *mut *mut CREDENTIALW) -> i32;
    fn CredWriteW(cred: *const CREDENTIALW, flags: u32) -> i32;
    fn CredFree(buffer: *mut std::ffi::c_void);
}

#[link(name = "user32")]
unsafe extern "system" {
    fn VkKeyScanW(ch: u16) -> i16;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
    fn SetConsoleCP(wCodePageID: u32) -> i32;
    fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
    fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
    fn FreeConsole() -> i32;
    fn AttachConsole(dwProcessId: u32) -> i32;
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *mut std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn GetConsoleScreenBufferInfo(
        hConsoleOutput: *mut std::ffi::c_void,
        lpConsoleScreenBufferInfo: *mut CONSOLE_SCREEN_BUFFER_INFO,
    ) -> i32;
    fn ReadConsoleOutputCharacterW(
        hConsoleOutput: *mut std::ffi::c_void,
        lpCharacter: *mut u16,
        nLength: u32,
        dwReadCoord: COORD,
        lpNumberOfCharsRead: *mut u32,
    ) -> i32;
    fn WriteConsoleInputW(
        hConsoleInput: *mut std::ffi::c_void,
        lpBuffer: *const INPUT_RECORD,
        nLength: u32,
        lpNumberOfEventsWritten: *mut u32,
    ) -> i32;
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> *mut std::ffi::c_void;
    fn Process32FirstW(hSnapshot: *mut std::ffi::c_void, lppe: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(hSnapshot: *mut std::ffi::c_void, lppe: *mut PROCESSENTRY32W) -> i32;
    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
    fn TerminateProcess(hProcess: *mut std::ffi::c_void, uExitCode: u32) -> i32;
    fn WaitForSingleObject(hHandle: *mut std::ffi::c_void, dwMilliseconds: u32) -> u32;
    fn GetCurrentProcessId() -> u32;
}

const STD_INPUT_HANDLE: u32 = 0xFFFFFFF6; // -10
const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5; // -11
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const SYNCHRONIZE: u32 = 0x00100000;
const PROCESS_TERMINATE: u32 = 0x0001;
const WAIT_OBJECT_0: u32 = 0;
const DETACHED_PROCESS: u32 = 0x00000008;
const GENERIC_READ: u32 = 0x80000000;
const GENERIC_WRITE: u32 = 0x40000000;
const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const OPEN_EXISTING: u32 = 3;

unsafe fn make_key_record(vk: u16, ch: u16, ctrl: u32, down: bool) -> INPUT_RECORD {
    let mut rec: INPUT_RECORD = std::mem::zeroed();
    rec.event_type = 1;
    rec.key_event.b_key_down = if down { 1 } else { 0 };
    rec.key_event.w_repeat_count = 1;
    rec.key_event.w_virtual_key_code = vk;
    rec.key_event.u_char = ch;
    rec.key_event.dw_control_key_state = if down { ctrl } else { 0 };
    rec
}

unsafe fn open_console_input() -> (*mut std::ffi::c_void, bool) {
    let conin_name = to_wide("CONIN$");
    let h = CreateFileW(
        conin_name.as_ptr(),
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        std::ptr::null_mut(),
        OPEN_EXISTING,
        0,
        std::ptr::null_mut(),
    );
    if !h.is_null() && h != (-1isize as *mut std::ffi::c_void) {
        (h, true)
    } else {
        (GetStdHandle(STD_INPUT_HANDLE), false)
    }
}

unsafe fn inject_keys_to_console(con_handle: *mut std::ffi::c_void, records: &[INPUT_RECORD]) -> bool {
    let mut written = 0u32;
    WriteConsoleInputW(con_handle, records.as_ptr(), records.len() as u32, &mut written) != 0
}

unsafe fn send_string_to_console(h_in: *mut std::ffi::c_void, text: &str) {
    let mut records: Vec<INPUT_RECORD> = Vec::new();
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        let (vk, ctrl) = if ch == '\r' {
            (0x0Du16, 0u32)
        } else {
            let scan = VkKeyScanW(ch as u16);
            let v = (scan & 0xFF) as u16;
            let shift = ((scan >> 8) & 0xFF) as u32;
            let mut c = 0u32;
            if (shift & 1) != 0 {
                c |= 0x0010; // SHIFT_PRESSED
            }
            if (shift & 2) != 0 {
                c |= 0x0008; // LEFT_CTRL_PRESSED
            }
            if (shift & 4) != 0 {
                c |= 0x0002; // LEFT_ALT_PRESSED
            }
            (v, c)
        };

        records.push(make_key_record(vk, ch as u16, ctrl, true));
        records.push(make_key_record(vk, ch as u16, 0, false));
    }
    inject_keys_to_console(h_in, &records);
}

fn get_conversation_id_from_log(agy_pid: u32) -> Option<String> {
    let log_dir = get_gemini_dir().join("antigravity-cli").join("log");
    if !log_dir.exists() {
        return None;
    }
    let pid_needle = format!("with pid {}", agy_pid);
    let conv_re = regex::Regex::new(r"(?:Resuming conversation|found conversation|update stream for)\s+([a-f0-9\-]{36})").ok()?;

    if let Ok(entries) = fs::read_dir(log_dir) {
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|x| x.path()))
            .filter(|p| p.extension().map(|ext| ext == "log").unwrap_or(false))
            .collect();
        files.sort_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
        files.reverse();

        for path in files.into_iter().take(20) {
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains(&pid_needle) {
                    if let Some(caps) = conv_re.captures(&content) {
                        if let Some(m) = caps.get(1) {
                            return Some(m.as_str().to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

unsafe fn run_worker_inject(target_pid: u32, action: &str) -> i32 {
    FreeConsole();
    if AttachConsole(target_pid) == 0 {
        return 1;
    }

    let (h_in, need_close) = open_console_input();
    if h_in.is_null() || h_in == (-1isize as *mut std::ffi::c_void) {
        FreeConsole();
        return 2;
    }

    if action == "exit" {
        // 1. 发送 Ctrl+C 打断当前可能正在进行的输出或输入状态
        let c_down = make_key_record(0x43, 3, 0x0008, true);
        let c_up = make_key_record(0x43, 3, 0, false);
        inject_keys_to_console(h_in, &[c_down, c_up]);
        std::thread::sleep(Duration::from_millis(100));

        // 2. 发送两次 Ctrl+D (cli.exit 标准热键)
        let d_down = make_key_record(0x44, 4, 0x0008, true);
        let d_up = make_key_record(0x44, 4, 0, false);
        inject_keys_to_console(h_in, &[d_down, d_up]);
        std::thread::sleep(Duration::from_millis(50));
        inject_keys_to_console(h_in, &[d_down, d_up]);
        std::thread::sleep(Duration::from_millis(100));
    } else if action.starts_with("resume") {
        let mut conv_id = None;
        if let Some(pos) = action.find(':') {
            let id = &action[pos + 1..];
            if !id.is_empty() {
                conv_id = Some(id.to_string());
            }
        }

        // 若未显式传入 ID，则尝试从当前控制台屏幕缓冲区正则抓取 agy --conversation=<id>
        if conv_id.is_none() {
            let conout_name = to_wide("CONOUT$");
            let h_out = CreateFileW(
                conout_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if !h_out.is_null() && h_out != (-1isize as *mut std::ffi::c_void) {
                let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
                if GetConsoleScreenBufferInfo(h_out, &mut csbi) != 0 {
                    let width = csbi.dw_size.x as usize;
                    let height = csbi.dw_size.y as usize;
                    let total = width * height;
                    if total > 0 && total <= 100_000 {
                        let mut buf = vec![0u16; total];
                        let mut read = 0u32;
                        let coord = COORD { x: 0, y: 0 };
                        if ReadConsoleOutputCharacterW(h_out, buf.as_mut_ptr(), total as u32, coord, &mut read) != 0 {
                            let raw = String::from_utf16_lossy(&buf[..read as usize]);
                            if let Ok(re) = regex::Regex::new(r"agy\s+--conversation[=\s]([a-f0-9\-]{36})") {
                                if let Some(caps) = re.captures(&raw) {
                                    if let Some(m) = caps.get(1) {
                                        conv_id = Some(m.as_str().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                CloseHandle(h_out);
            }
        }

        let resume_cmd = match conv_id {
            Some(id) => format!("agy --conversation={}\r", id),
            None => "agy -c\r".to_string(),
        };
        send_string_to_console(h_in, &resume_cmd);
    } else {
        let mut cmd = action.to_string();
        if !cmd.ends_with('\r') {
            cmd.push('\r');
        }
        send_string_to_console(h_in, &cmd);
    }

    if need_close {
        CloseHandle(h_in);
    }
    FreeConsole();

    0
}

fn get_all_agy_processes() -> Vec<(u32, u32)> {
    let mut agy_list = Vec::new();
    let mut parent_map: HashMap<u32, u32> = HashMap::new();
    let my_pid = unsafe { GetCurrentProcessId() };

    unsafe {
        let h_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if h_snap.is_null() || h_snap == (-1isize as *mut std::ffi::c_void) {
            return agy_list;
        }

        let mut pe: PROCESSENTRY32W = std::mem::zeroed();
        pe.dw_size = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(h_snap, &mut pe) != 0 {
            loop {
                let pid = pe.th32_process_id;
                let ppid = pe.th32_parent_process_id;
                parent_map.insert(pid, ppid);

                let len = pe.sz_exe_file.iter().position(|&c| c == 0).unwrap_or(pe.sz_exe_file.len());
                let exe_name = String::from_utf16_lossy(&pe.sz_exe_file[..len]);
                if exe_name.eq_ignore_ascii_case("agy.exe") {
                    agy_list.push((pid, ppid));
                }

                if Process32NextW(h_snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(h_snap);
    }

    // 收集当前进程的所有祖先 PID (Ancestor chain)
    let mut ancestors = HashSet::new();
    let mut cur = my_pid;
    while let Some(&p) = parent_map.get(&cur) {
        if p == 0 || p == cur || ancestors.contains(&p) {
            break;
        }
        ancestors.insert(p);
        cur = p;
    }

    // 过滤掉当前进程自身以及祖先树上的 agy.exe
    // 防止如果配额中心是从某个 agy 终端内作为子进程启动时，自我终止导致整个调用链崩溃
    agy_list.into_iter().filter(|(pid, _)| *pid != my_pid && !ancestors.contains(pid)).collect()
}

fn reload_all_active_agy_sessions() -> usize {
    let procs = get_all_agy_processes();
    if procs.is_empty() {
        return 0;
    }

    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return 0,
    };

    println!("\n{}[*] 检测到 {} 个运行中的 agy 终端会话，正在下发协同热重载退出与精准独立会话恢复...{}", C_DIM, procs.len(), C_RESET);
    let mut reloaded = 0;

    for (agy_pid, shell_pid) in procs {
        // 提前通过日志或进程映射解析其当前专属的 Conversation ID
        let log_conv_id = get_conversation_id_from_log(agy_pid);

        // 1. 尝试注入退出控制符 (Ctrl+C, Ctrl+D)
        let _ = Command::new(&exe_path)
            .args(["__worker_inject", &agy_pid.to_string(), "exit"])
            .creation_flags(DETACHED_PROCESS)
            .status();

        // 2. 等待原 agy.exe 退出，给与 1.2 秒优雅退出窗口
        let mut exited = false;
        unsafe {
            let h_proc = OpenProcess(SYNCHRONIZE | PROCESS_TERMINATE, 0, agy_pid);
            if !h_proc.is_null() && h_proc != (-1isize as *mut std::ffi::c_void) {
                let wait_res = WaitForSingleObject(h_proc, 1200);
                if wait_res == WAIT_OBJECT_0 {
                    exited = true;
                } else {
                    // 若超过 1.2 秒未退出，执行兜底强行终止 (Fallback Kill)
                    // agy 每一轮对话和工具调用均已实时持久化到 SQLite 和 history.jsonl，强制终止不会丢失任何会话历史
                    let _ = TerminateProcess(h_proc, 0);
                    let wait_kill = WaitForSingleObject(h_proc, 500);
                    if wait_kill == WAIT_OBJECT_0 {
                        exited = true;
                    }
                }
                CloseHandle(h_proc);
            } else {
                exited = true;
            }
        }

        if exited {
            // 稍作等待 400ms，确保宿主 Shell (PowerShell/CMD) 恢复命令输入提示符 (Prompt)
            std::thread::sleep(Duration::from_millis(400));

            // 3. 构造精准 resume 参数（传递解析到的专属会话 ID，如未识别则交由 worker 从屏幕缓冲区抓取或回退 -c）
            let resume_arg = match &log_conv_id {
                Some(id) => format!("resume:{}", id),
                None => "resume".to_string(),
            };

            let res = Command::new(&exe_path)
                .args(["__worker_inject", &shell_pid.to_string(), &resume_arg])
                .creation_flags(DETACHED_PROCESS)
                .status();

            if let Ok(st) = res {
                if st.success() {
                    reloaded += 1;
                    if let Some(id) = &log_conv_id {
                        let short_id = if id.len() >= 8 { &id[..8] } else { id.as_str() };
                        println!("{}[+] 会话 (PID: {}) 已成功精准恢复专属会话 [{}]！{}", C_GREEN, agy_pid, short_id, C_RESET);
                    } else {
                        println!("{}[+] 会话 (PID: {}) 已成功下发热重载并恢复！{}", C_GREEN, agy_pid, C_RESET);
                    }
                } else {
                    println!("{}[警告] 向宿主 Shell (PID: {}) 下发恢复指令异常。{}", C_YELLOW, shell_pid, C_RESET);
                }
            } else {
                println!("{}[警告] 向宿主 Shell (PID: {}) 发送恢复指令失败。{}", C_YELLOW, shell_pid, C_RESET);
            }
        } else {
            println!("{}[警告] 会话 (PID: {}) 退出超时且强行终止失败，已跳过。{}", C_YELLOW, agy_pid, C_RESET);
        }
    }

    reloaded
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn init_console() {
    unsafe {
        SetConsoleOutputCP(65001);
        SetConsoleCP(65001);
        let h_out = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode: u32 = 0;
        if GetConsoleMode(h_out, &mut mode) != 0 {
            SetConsoleMode(h_out, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}

fn pause() {
    print!("\n{}按回车键返回主菜单...{}", C_DIM, C_RESET);
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").to_string()
}

fn visible_width(s: &str) -> usize {
    strip_ansi(s).width()
}

fn pad_visual(s: &str, target_width: usize) -> String {
    let w = visible_width(s);
    if w >= target_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target_width - w))
    }
}

fn pad_visual_right(s: &str, target_width: usize) -> String {
    let w = visible_width(s);
    if w >= target_width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(target_width - w), s)
    }
}

fn make_bar(fraction: f64, width: usize) -> String {
    let f = fraction.clamp(0.0, 1.0);
    let filled = (f * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    let color = if f >= 0.50 {
        C_GREEN
    } else if f >= 0.20 {
        C_YELLOW
    } else {
        C_RED
    };

    let bar_filled = "█".repeat(filled);
    let bar_empty = "░".repeat(empty);
    let pct = format!("{:5.1}%", f * 100.0);
    format!("[{}{}{}{}{}] {}{}{}", color, bar_filled, C_DIM, bar_empty, C_RESET, color, pct, C_RESET)
}

// Keyring I/O
fn read_keyring() -> Option<Value> {
    let target = to_wide("gemini:antigravity");
    let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        if CredReadW(target.as_ptr(), 1, 0, &mut cred_ptr) != 0 && !cred_ptr.is_null() {
            let cred = &*cred_ptr;
            let slice = std::slice::from_raw_parts(cred.credential_blob, cred.credential_blob_size as usize);
            let val = serde_json::from_slice(slice).ok();
            CredFree(cred_ptr as *mut std::ffi::c_void);
            return val;
        }
    }
    None
}

fn write_keyring(data: &Value) -> bool {
    let json_bytes = match serde_json::to_vec(data) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut target = to_wide("gemini:antigravity");
    let mut comment = to_wide("Antigravity CLI credentials");
    let mut user = to_wide("antigravity");

    let cred = CREDENTIALW {
        flags: 0,
        cred_type: 1, // CRED_TYPE_GENERIC
        target_name: target.as_mut_ptr(),
        comment: comment.as_mut_ptr(),
        last_written: [0, 0],
        credential_blob_size: json_bytes.len() as u32,
        credential_blob: json_bytes.as_ptr() as *mut u8,
        persist: 2, // CRED_PERSIST_LOCAL_MACHINE
        attribute_count: 0,
        attributes: std::ptr::null_mut(),
        target_alias: std::ptr::null_mut(),
        user_name: user.as_mut_ptr(),
    };

    unsafe { CredWriteW(&cred, 0) != 0 }
}

fn get_gemini_dir() -> PathBuf {
    dirs::home_dir().map(|h| h.join(".gemini")).unwrap_or_else(|| PathBuf::from("."))
}

fn get_accounts_dir() -> PathBuf {
    let d = get_gemini_dir().join("accounts");
    let _ = fs::create_dir_all(&d);
    d
}

fn get_index_path() -> PathBuf {
    get_gemini_dir().join("accounts.json")
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AccountSummary {
    id: String,
    email: String,
    name: Option<String>,
    disabled: Option<bool>,
    proxy_disabled: Option<bool>,
    created_at: Option<i64>,
    last_used: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AccountIndex {
    version: Option<String>,
    accounts: Vec<AccountSummary>,
    current_account_id: Option<String>,
    current_target_ide: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TokenInfo {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    expiry_timestamp: Option<i64>,
    token_type: Option<String>,
    email: Option<String>,
    is_gcp_tos: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AccountDetail {
    id: String,
    email: String,
    name: Option<String>,
    token: Option<TokenInfo>,
    created_at: Option<i64>,
    last_used: Option<i64>,
    disabled: Option<bool>,
    proxy_disabled: Option<bool>,
}

fn load_account_index() -> AccountIndex {
    let p = get_index_path();
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(idx) = serde_json::from_str::<AccountIndex>(&content) {
                return idx;
            }
        }
    }
    AccountIndex {
        version: Some("1".to_string()),
        accounts: Vec::new(),
        current_account_id: None,
        current_target_ide: Some("agy".to_string()),
    }
}

fn save_account_index(idx: &AccountIndex) {
    let p = get_index_path();
    if let Ok(json) = serde_json::to_string_pretty(idx) {
        let _ = fs::write(p, json);
    }
}

fn load_account(id: &str) -> Option<AccountDetail> {
    let p = get_accounts_dir().join(format!("{}.json", id));
    if p.exists() {
        if let Ok(content) = fs::read_to_string(p) {
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

fn save_account(acc: &AccountDetail) {
    let p = get_accounts_dir().join(format!("{}.json", acc.id));
    if let Ok(json) = serde_json::to_string_pretty(acc) {
        let _ = fs::write(p, json);
    }
}

// Native curl helper
fn curl_post_json(url: &str, headers: &[(&str, &str)], body: &str) -> Result<String, String> {
    let mut cmd = Command::new("curl.exe");
    cmd.args(["-s", "-m", "15", "-X", "POST", url]);
    for (k, v) in headers {
        cmd.args(["-H", &format!("{}: {}", k, v)]);
    }
    if !body.is_empty() {
        cmd.args(["-d", body]);
    }
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

fn curl_get_json(url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
    let mut cmd = Command::new("curl.exe");
    cmd.args(["-s", "-m", "15", url]);
    for (k, v) in headers {
        cmd.args(["-H", &format!("{}: {}", k, v)]);
    }
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

fn refresh_oauth_token(refresh_token: &str) -> Result<(String, i64), String> {
    let body = format!(
        "client_id={}&client_secret={}&grant_type=refresh_token&refresh_token={}",
        OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, refresh_token
    );
    let res = curl_post_json(
        "https://oauth2.googleapis.com/token",
        &[("Content-Type", "application/x-www-form-urlencoded")],
        &body,
    )?;
    let val: Value = serde_json::from_str(&res).map_err(|e| e.to_string())?;
    if let Some(tok) = val.get("access_token").and_then(|t| t.as_str()) {
        let exp = val.get("expires_in").and_then(|x| x.as_i64()).unwrap_or(3600);
        Ok((tok.to_string(), exp))
    } else {
        let err = val.get("error_description").or_else(|| val.get("error")).and_then(|x| x.as_str()).unwrap_or("Unknown refresh error");
        Err(err.to_string())
    }
}

fn get_valid_active_token(force_refresh: bool) -> Result<String, String> {
    let cred_opt = read_keyring();
    if let Some(mut cred_data) = cred_opt {
        let tok_obj = cred_data.get("token").cloned().unwrap_or(Value::Null);
        let mut access_token = tok_obj.get("access_token").and_then(|t| t.as_str()).unwrap_or("").to_string();
        let refresh_token = tok_obj.get("refresh_token").and_then(|t| t.as_str()).unwrap_or("").to_string();
        let expiry_str = tok_obj.get("expiry").and_then(|t| t.as_str()).unwrap_or("");

        let mut is_expired = force_refresh || access_token.is_empty();
        if !is_expired && !expiry_str.is_empty() {
            if let Ok(dt) = DateTime::parse_from_rfc3339(expiry_str) {
                let now = Utc::now();
                if (dt.signed_duration_since(now)).num_seconds() < 45 {
                    is_expired = true;
                }
            }
        }

        if is_expired && !refresh_token.is_empty() {
            if let Ok((new_tok, _)) = refresh_oauth_token(&refresh_token) {
                access_token = new_tok.clone();
                let now_dt = Utc::now().to_rfc3339();
                if let Some(obj) = cred_data.get_mut("token").and_then(|t| t.as_object_mut()) {
                    obj.insert("access_token".to_string(), Value::String(new_tok));
                    obj.insert("expiry".to_string(), Value::String(now_dt));
                    write_keyring(&cred_data);
                }
            }
        }

        if !access_token.is_empty() {
            return Ok(access_token);
        }
    }

    // Fallback: search accounts pool
    let idx = load_account_index();
    if let Some(curr_id) = idx.current_account_id {
        if let Some(mut acc) = load_account(&curr_id) {
            return ensure_valid_token(&mut acc);
        }
    }

    Err("未能获取到有效的 Antigravity 凭证".to_string())
}

fn ensure_valid_token(acc: &mut AccountDetail) -> Result<String, String> {
    let now = Utc::now().timestamp();
    let tok_info = acc.token.as_ref().ok_or("Account has no token info")?;
    let access_tok = tok_info.access_token.clone().unwrap_or_default();
    let refresh_tok = tok_info.refresh_token.clone().unwrap_or_default();
    let expiry_ts = tok_info.expiry_timestamp.unwrap_or(0);

    if !access_tok.is_empty() && (expiry_ts - now) > 60 {
        return Ok(access_tok);
    }

    if refresh_tok.is_empty() {
        return Err("No refresh token available".to_string());
    }

    let (new_tok, exp) = refresh_oauth_token(&refresh_tok)?;
    if let Some(ref mut t) = acc.token {
        t.access_token = Some(new_tok.clone());
        t.expires_in = Some(exp);
        t.expiry_timestamp = Some(now + exp);
    }
    save_account(acc);
    Ok(new_tok)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct ParsedQuota {
    gemini_5h: Option<f64>,
    gemini_weekly: Option<f64>,
    gemini_5h_reset: Option<String>,
    gemini_5h_countdown: Option<String>,
    gemini_weekly_reset: Option<String>,
    gemini_weekly_countdown: Option<String>,
    claude_5h: Option<f64>,
    claude_weekly: Option<f64>,
    claude_5h_reset: Option<String>,
    claude_5h_countdown: Option<String>,
    claude_weekly_reset: Option<String>,
    claude_weekly_countdown: Option<String>,
    effective_score: f64,
}

fn format_compact_countdown(target_iso: Option<&str>, frac: Option<f64>) -> String {
    if let Some(f) = frac {
        if f >= 0.999 {
            return "满额".to_string();
        }
    }
    let iso = match target_iso {
        Some(s) if !s.is_empty() => s,
        _ => return "--".to_string(),
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso) {
        let now = Utc::now();
        let diff = dt.signed_duration_since(now).num_seconds();
        if diff <= 0 {
            return "已就绪".to_string();
        }
        let hours = diff / 3600;
        let mins = (diff % 3600) / 60;
        if hours >= 24 {
            let days = hours / 24;
            let rem_h = hours % 24;
            if rem_h > 0 {
                format!("{}天{:02}h", days, rem_h)
            } else {
                format!("{}天", days)
            }
        } else if hours > 0 {
            format!("{}h{:02}m", hours, mins)
        } else {
            format!("{}分", mins)
        }
    } else {
        "--".to_string()
    }
}

fn format_countdown_pair(cd_5h: Option<&str>, cd_w: Option<&str>) -> String {
    let s_5h = cd_5h.unwrap_or("--");
    let s_w = cd_w.unwrap_or("--");
    format!("{} / {}", pad_visual_right(s_5h, 6), pad_visual_right(s_w, 6))
}

fn format_relative_time(iso_str: &str) -> (String, String) {
    if iso_str.is_empty() {
        return ("N/A".to_string(), "N/A".to_string());
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso_str) {
        let now = Utc::now();
        let diff = dt.signed_duration_since(now).num_seconds();
        let local_str = dt.with_timezone(&Local).format("%m-%d %H:%M").to_string();

        if diff <= 0 {
            return ("已重置".to_string(), local_str);
        }

        let days = diff / 86400;
        let hours = (diff % 86400) / 3600;
        let mins = (diff % 3600) / 60;

        let countdown = if days > 0 {
            format!("{}天 {}小时后", days, hours)
        } else if hours > 0 {
            format!("{}小时 {}分后", hours, mins)
        } else {
            format!("{}分钟后", mins)
        };
        (countdown, local_str)
    } else {
        (iso_str.to_string(), iso_str.to_string())
    }
}

fn parse_quota_buckets(summary: &Value) -> ParsedQuota {
    let mut q = ParsedQuota::default();
    if let Some(groups) = summary.get("groups").and_then(|g| g.as_array()) {
        for g in groups {
            let gname = g.get("displayName").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
            let is_claude = gname.contains("claude") || gname.contains("gpt");
            if let Some(buckets) = g.get("buckets").and_then(|b| b.as_array()) {
                for b in buckets {
                    let win = b.get("window").and_then(|x| x.as_str()).unwrap_or("");
                    let frac = b.get("remainingFraction").and_then(|x| x.as_f64());
                    let rt = b.get("resetTime").and_then(|x| x.as_str()).map(|s| s.to_string());
                    if is_claude {
                        if win == "5h" {
                            q.claude_5h = frac;
                            q.claude_5h_reset = rt;
                        } else if win == "weekly" {
                            q.claude_weekly = frac;
                            q.claude_weekly_reset = rt;
                        }
                    } else {
                        if win == "5h" {
                            q.gemini_5h = frac;
                            q.gemini_5h_reset = rt;
                        } else if win == "weekly" {
                            q.gemini_weekly = frac;
                            q.gemini_weekly_reset = rt;
                        }
                    }
                }
            }
        }
    }

    q.gemini_5h_countdown = Some(format_compact_countdown(q.gemini_5h_reset.as_deref(), q.gemini_5h));
    q.gemini_weekly_countdown = Some(format_compact_countdown(q.gemini_weekly_reset.as_deref(), q.gemini_weekly));
    q.claude_5h_countdown = Some(format_compact_countdown(q.claude_5h_reset.as_deref(), q.claude_5h));
    q.claude_weekly_countdown = Some(format_compact_countdown(q.claude_weekly_reset.as_deref(), q.claude_weekly));

    let g_eff = q.gemini_5h.unwrap_or(0.0).min(q.gemini_weekly.unwrap_or(0.0));
    let c_eff = q.claude_5h.unwrap_or(0.0).min(q.claude_weekly.unwrap_or(0.0));
    q.effective_score = g_eff * 0.4 + c_eff * 0.6;

    q
}

fn call_api(endpoint: &str, token: &str, body: &str) -> Result<Value, String> {
    let url = format!("{}:{}", API_BASE, endpoint);
    let headers = [
        ("Content-Type", "application/json"),
        ("Authorization", &format!("Bearer {}", token)),
        ("User-Agent", USER_AGENT),
    ];
    let res = curl_post_json(&url, &headers, body)?;
    serde_json::from_str(&res).map_err(|e| format!("JSON解析错误: {} (原始: {})", e, res))
}

fn fetch_quota_summary(token: &str) -> Result<Value, String> {
    call_api("retrieveUserQuotaSummary", token, "{}")
}

fn fetch_userinfo(access_token: &str) -> Result<(String, String), String> {
    let url = "https://www.googleapis.com/oauth2/v3/userinfo";
    let auth_val = format!("Bearer {}", access_token);
    let headers = [
        ("Authorization", auth_val.as_str()),
        ("User-Agent", USER_AGENT),
    ];
    let res = curl_get_json(url, &headers)?;
    let val: Value = serde_json::from_str(&res).map_err(|e| e.to_string())?;
    let email = val.get("email").and_then(|x| x.as_str()).unwrap_or("unknown@gmail.com").to_string();
    let name = val.get("name").and_then(|x| x.as_str()).unwrap_or_else(|| email.split('@').next().unwrap_or("User")).to_string();
    Ok((email, name))
}

fn get_current_active_info() -> (String, String) {
    let mut email = "未知账号".to_string();
    let tier = "Antigravity".to_string();
    let ga_file = get_gemini_dir().join("google_accounts.json");
    if ga_file.exists() {
        if let Ok(c) = fs::read_to_string(&ga_file) {
            if let Ok(v) = serde_json::from_str::<Value>(&c) {
                if let Some(act) = v.get("active").and_then(|x| x.as_str()) {
                    email = act.to_string();
                }
            }
        }
    }
    (email, tier)
}

fn switch_account_by_id(target_id: &str) -> Result<(String, usize), String> {
    let mut acc = load_account(target_id).ok_or("找不到指定账号详情")?;
    let access_tok = ensure_valid_token(&mut acc)?;
    let refresh_tok = acc.token.as_ref().and_then(|t| t.refresh_token.clone()).unwrap_or_default();

    let mut cred_obj = read_keyring().unwrap_or(serde_json::json!({
        "account_id": "",
        "token": {
            "access_token": "",
            "refresh_token": "",
            "expiry": "",
            "token_type": "Bearer"
        }
    }));

    let now_dt = Utc::now().to_rfc3339();
    cred_obj["account_id"] = Value::String(acc.id.clone());
    if let Some(t) = cred_obj.get_mut("token").and_then(|x| x.as_object_mut()) {
        t.insert("access_token".to_string(), Value::String(access_tok.clone()));
        t.insert("refresh_token".to_string(), Value::String(refresh_tok.clone()));
        t.insert("expiry".to_string(), Value::String(now_dt.clone()));
    }

    if !write_keyring(&cred_obj) {
        return Err("写入 Windows 凭据管理器失败".to_string());
    }

    let ga_file = get_gemini_dir().join("google_accounts.json");
    let ga_obj = serde_json::json!({ "active": acc.email });
    let _ = fs::write(ga_file, serde_json::to_string_pretty(&ga_obj).unwrap_or_default());

    // 同步 ~/.gemini/oauth_creds.json
    let oc_file = get_gemini_dir().join("oauth_creds.json");
    let expiry_ms = acc.token.as_ref().and_then(|t| t.expiry_timestamp).unwrap_or(0) * 1000;
    let oc_obj = serde_json::json!({
        "access_token": access_tok,
        "refresh_token": refresh_tok,
        "token_type": "Bearer",
        "expiry_date": expiry_ms
    });
    let _ = fs::write(oc_file, serde_json::to_string_pretty(&oc_obj).unwrap_or_default());

    // 同步 ~/.gemini/antigravity-cli/antigravity-oauth-token (备用容灾文件)
    let cli_dir = get_gemini_dir().join("antigravity-cli");
    if cli_dir.exists() {
        let cli_tok_file = cli_dir.join("antigravity-oauth-token");
        let cli_tok_obj = serde_json::json!({
            "token": {
                "access_token": access_tok,
                "token_type": "Bearer",
                "refresh_token": refresh_tok,
                "expiry": now_dt
            },
            "auth_method": "consumer"
        });
        let _ = fs::write(cli_tok_file, serde_json::to_string(&cli_tok_obj).unwrap_or_default());
    }

    let idx = load_account_index();
    let mut new_idx = idx.clone();
    new_idx.current_account_id = Some(acc.id.clone());
    new_idx.current_target_ide = Some("agy".to_string());
    let now = Utc::now().timestamp();
    for s in &mut new_idx.accounts {
        if s.id == acc.id {
            s.last_used = Some(now);
        }
    }
    save_account_index(&new_idx);

    acc.last_used = Some(now);
    save_account(&acc);

    // 自动通知当前所有运行中的 agy 会话优雅退出并 resume 继续当前会话
    let reloaded_count = reload_all_active_agy_sessions();

    Ok((acc.email, reloaded_count))
}

// -----------------------------------------------------------------------------
// Options Implementations
// -----------------------------------------------------------------------------

// [1] 查看当前活动账号额度详情
fn option_view_current_quota() {
    clear_screen();
    println!("\n{}[*] 正在拉取当前活动账号实时配额详情...{}", C_DIM, C_RESET);
    let token = match get_valid_active_token(false) {
        Ok(t) => t,
        Err(e) => {
            println!("\n{}[-] 获取凭据失败: {}{}", C_RED, e, C_RESET);
            pause();
            return;
        }
    };

    let summary = match fetch_quota_summary(&token) {
        Ok(s) => s,
        Err(e) => {
            println!("\n{}[-] 查询配额失败: {}{}", C_RED, e, C_RESET);
            pause();
            return;
        }
    };

    let code_assist = call_api("loadCodeAssist", &token, "{}").ok();
    let (email, mut tier) = get_current_active_info();
    if let Some(ca) = &code_assist {
        if let Some(tname) = ca.get("currentTier").and_then(|x| x.get("name")).and_then(|x| x.as_str()) {
            tier = tname.to_string();
        }
    }

    clear_screen();
    println!("{}================================================================{}", C_BOLD, C_RESET);
    println!("{} >>> Antigravity CLI Usage & Quota Monitor (AGY 额度实时看板) <<<{}", C_BOLD, C_RESET);
    println!("{}================================================================{}", C_BOLD, C_RESET);
    println!(" 账号: {}{}{}  |  套餐: {}{}{}", C_WHITE, email, C_RESET, C_YELLOW, tier, C_RESET);
    println!(" 时间: {}{}{}", C_DIM, Local::now().format("%Y-%m-%d %H:%M:%S"), C_RESET);
    println!();

    if let Some(groups) = summary.get("groups").and_then(|g| g.as_array()) {
        for g in groups {
            let title = g.get("displayName").and_then(|x| x.as_str()).unwrap_or("Group");
            let desc = g.get("description").and_then(|x| x.as_str()).unwrap_or("");
            let is_gemini = title.to_lowercase().contains("gemini");
            let (tag, hdr_color) = if is_gemini {
                ("[Gemini Models]", C_BLUE)
            } else {
                ("[Claude / GPT Models]", C_MAGENTA)
            };

            println!("{}{}{} {}  {}({}){}", C_BOLD, hdr_color, tag, title, C_DIM, desc, C_RESET);
            println!("  {}", "-".repeat(60));

            if let Some(buckets) = g.get("buckets").and_then(|b| b.as_array()) {
                let mut sorted_buckets = buckets.clone();
                sorted_buckets.sort_by_key(|b| {
                    match b.get("window").and_then(|x| x.as_str()).unwrap_or("") {
                        "5h" => 0,
                        "weekly" => 1,
                        _ => 2,
                    }
                });

                for b in &sorted_buckets {
                    let win = b.get("window").and_then(|x| x.as_str()).unwrap_or("");
                    let frac = b.get("remainingFraction").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    let rt = b.get("resetTime").and_then(|x| x.as_str()).unwrap_or("");
                    let (countdown, local_time) = format_relative_time(rt);

                    let label = match win {
                        "5h" => "  [5小时限额  ]",
                        "weekly" => "  [每周限额    ]",
                        _ => "  [模型配额    ]",
                    };

                    let bar = make_bar(frac, 22);
                    let reset_hint = if frac < 0.999 && !rt.is_empty() {
                        format!(" {}(刷新: {} | 本地: {}){}", C_DIM, countdown, local_time, C_RESET)
                    } else if !rt.is_empty() {
                        format!(" {}(满额 | 周期重置: {} | 本地: {}){}", C_DIM, countdown, local_time, C_RESET)
                    } else {
                        String::new()
                    };
                    println!("{} : {}{}", label, bar, reset_hint);
                }
            }
            println!();
        }
    }

    println!("{}[说明]: 所有 Gemini 模型共享上述同一个配额池，Claude 模型共享另一个配额池。{}", C_DIM, C_RESET);
    print!("\n>> 按回车返回主菜单 (或输入 {}'v'{} 查看展开的具体模型列表): ", C_BOLD, C_RESET);
    let _ = io::stdout().flush();
    let mut sub_in = String::new();
    let _ = io::stdin().read_line(&mut sub_in);

    if sub_in.trim().eq_ignore_ascii_case("v") {
        clear_screen();
        println!("\n{}[*] 正在加载具体模型映射明细...{}", C_DIM, C_RESET);
        if let Ok(uq) = call_api("retrieveUserQuota", &token, "{}") {
            clear_screen();
            println!("{}=== 具体各模型限额与配额组映射明细 ==={}", C_BOLD, C_RESET);
            if let Some(models) = uq.get("models").and_then(|m| m.as_array()) {
                for m in models {
                    let mname = m.get("model").and_then(|x| x.as_str()).unwrap_or("unknown");
                    let gname = m.get("group").and_then(|x| x.as_str()).unwrap_or("-");
                    println!("  * {:36} -> 所属配额组: {}", mname, gname);
                }
            }
        }
        pause();
    }
}

// [2] 查看所有账号全局大盘
struct AccountRow {
    acc: AccountSummary,
    parsed: Option<ParsedQuota>,
    err: Option<String>,
}

fn fetch_all_accounts_data() -> (Vec<AccountRow>, Option<String>, Option<AccountSummary>) {
    let idx = load_account_index();
    let current_id = idx.current_account_id.clone();
    let mut rows = Vec::new();

    for a in &idx.accounts {
        let sid = a.id.clone();
        let (parsed, err) = if let Some(mut acc) = load_account(&sid) {
            match ensure_valid_token(&mut acc) {
                Ok(tok) => match fetch_quota_summary(&tok) {
                    Ok(sum) => (Some(parse_quota_buckets(&sum)), None),
                    Err(e) => (None, Some(e)),
                },
                Err(e) => (None, Some(e)),
            }
        } else {
            (None, Some("文件丢失".to_string()))
        };
        rows.push(AccountRow {
            acc: a.clone(),
            parsed,
            err,
        });
    }

    let mut best_score = -1.0;
    let mut best_acc: Option<AccountSummary> = None;
    for r in &rows {
        let is_cur = current_id.as_deref() == Some(&r.acc.id);
        if !is_cur {
            if let Some(p) = &r.parsed {
                if p.effective_score > best_score {
                    best_score = p.effective_score;
                    best_acc = Some(r.acc.clone());
                }
            }
        }
    }

    (rows, current_id, best_acc)
}

fn print_accounts_table(rows: &[AccountRow], current_id: Option<&str>, best_acc: Option<&AccountSummary>) {
    let header = format!(
        " {} {} {} {} {} {} {}",
        pad_visual("#", 4),
        pad_visual("状态", 8),
        pad_visual("邮箱账号", 24),
        pad_visual("Gemini (5h/周)", 15),
        pad_visual("G刷新(5h/周)", 16),
        pad_visual("Claude (5h/周)", 15),
        pad_visual("C刷新(5h/周)", 16)
    );
    println!("{}", header);
    println!("{}", "-".repeat(105));

    let fmt_pct = |val: Option<f64>| -> String {
        match val {
            Some(v) => {
                let pct = v * 100.0;
                if pct >= 50.0 {
                    format!("{}{:4.0}%{}", C_GREEN, pct, C_RESET)
                } else if pct >= 20.0 {
                    format!("{}{:4.0}%{}", C_YELLOW, pct, C_RESET)
                } else {
                    format!("{}{:4.0}%{}", C_RED, pct, C_RESET)
                }
            }
            None => format!("{}  -- {}", C_DIM, C_RESET),
        }
    };

    for (idx, r) in rows.iter().enumerate() {
        let is_cur = current_id == Some(&r.acc.id);
        let is_best = best_acc.as_ref().map(|b| b.id.as_str()) == Some(&r.acc.id) && !is_cur;

        let status = if is_cur {
            format!("{}[当前*]{}", C_GREEN, C_RESET)
        } else if is_best {
            format!("{}[推荐*]{}", C_MAGENTA, C_RESET)
        } else if let Some(e) = &r.err {
            if e.contains("403") {
                format!("{}[无许可]{}", C_RED, C_RESET)
            } else {
                format!("{}[异常]{}", C_YELLOW, C_RESET)
            }
        } else {
            format!("{}[可用]{}", C_CYAN, C_RESET)
        };

        let email_str = if is_cur {
            format!("{}{}{}{}", C_BOLD, C_WHITE, r.acc.email, C_RESET)
        } else if is_best {
            format!("{}{}{}", C_MAGENTA, r.acc.email, C_RESET)
        } else {
            r.acc.email.clone()
        };

        let (g_str, g_cd, c_str, c_cd) = if let Some(p) = &r.parsed {
            (
                format!("{} / {}", fmt_pct(p.gemini_5h), fmt_pct(p.gemini_weekly)),
                format_countdown_pair(p.gemini_5h_countdown.as_deref(), p.gemini_weekly_countdown.as_deref()),
                format!("{} / {}", fmt_pct(p.claude_5h), fmt_pct(p.claude_weekly)),
                format_countdown_pair(p.claude_5h_countdown.as_deref(), p.claude_weekly_countdown.as_deref()),
            )
        } else {
            (
                format!("{}  --  /  --  {}", C_DIM, C_RESET),
                format!("{}    -- /     --{}", C_DIM, C_RESET),
                format!("{}  --  /  --  {}", C_DIM, C_RESET),
                format!("{}    -- /     --{}", C_DIM, C_RESET),
            )
        };

        let row = format!(
            " {} {} {} {} {} {} {}",
            pad_visual(&format!("[{}]", idx), 4),
            pad_visual(&status, 8),
            pad_visual(&email_str, 24),
            pad_visual(&g_str, 15),
            pad_visual(&g_cd, 16),
            pad_visual(&c_str, 15),
            pad_visual(&c_cd, 16)
        );
        println!("{}", row);
    }
    println!("{}", "-".repeat(105));

    let normal_count = rows.iter().filter(|r| r.err.is_none()).count();
    let err_count = rows.len().saturating_sub(normal_count);

    let mut nearest_5h_email: Option<&str> = None;
    let mut nearest_5h_sec = i64::MAX;
    let mut nearest_5h_desc = String::new();

    let mut nearest_w_email: Option<&str> = None;
    let mut nearest_w_sec = i64::MAX;
    let mut nearest_w_desc = String::new();

    let now = Utc::now();

    for r in rows {
        if let Some(p) = &r.parsed {
            for (rt_opt, frac_opt, m_type) in [
                (p.gemini_5h_reset.as_deref(), p.gemini_5h, "Gemini 5h"),
                (p.claude_5h_reset.as_deref(), p.claude_5h, "Claude 5h"),
            ] {
                if let (Some(rt), Some(frac)) = (rt_opt, frac_opt) {
                    if frac < 0.999 && !rt.is_empty() {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(rt) {
                            let diff = dt.signed_duration_since(now).num_seconds();
                            if diff > 0 && diff < nearest_5h_sec {
                                nearest_5h_sec = diff;
                                nearest_5h_email = Some(&r.acc.email);
                                let (cd, loc) = format_relative_time(rt);
                                nearest_5h_desc = format!("{} 将于 {} ({}) 恢复", m_type, cd, loc);
                            }
                        }
                    }
                }
            }

            for (rt_opt, frac_opt, m_type) in [
                (p.gemini_weekly_reset.as_deref(), p.gemini_weekly, "Gemini 周额度"),
                (p.claude_weekly_reset.as_deref(), p.claude_weekly, "Claude 周额度"),
            ] {
                if let (Some(rt), Some(frac)) = (rt_opt, frac_opt) {
                    if frac < 0.999 && !rt.is_empty() {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(rt) {
                            let diff = dt.signed_duration_since(now).num_seconds();
                            if diff > 0 && diff < nearest_w_sec {
                                nearest_w_sec = diff;
                                nearest_w_email = Some(&r.acc.email);
                                let (cd, loc) = format_relative_time(rt);
                                nearest_w_desc = format!("{} 将于 {} ({}) 恢复", m_type, cd, loc);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("{}[统计] 账号池: 共 {} 个账号 | 正常可用: {} | 需授权/受限: {}{}", C_DIM, rows.len(), normal_count, err_count, C_RESET);
    if let Some(email) = nearest_5h_email {
        println!("  [*] 最近5h刷新: {}{}{}{} 的 {}{}{}", C_CYAN, C_BOLD, email, C_RESET, C_YELLOW, nearest_5h_desc, C_RESET);
    }
    if let Some(email) = nearest_w_email {
        println!("  [*] 最近周刷新: {}{}{}{} 的 {}{}{}", C_BLUE, C_BOLD, email, C_RESET, C_YELLOW, nearest_w_desc, C_RESET);
    }
    if let Some(best) = best_acc {
        println!("  [*] 智能推荐: 可用额度最充沛账号为 {}{}{}{}{}，输入 {}'auto'{} 可一键秒切！", C_MAGENTA, C_BOLD, C_WHITE, best.email, C_RESET, C_BOLD, C_RESET);
    }
    println!("{}[说明]: 生图功能 (Imagen 3 / gemini-3.1-flash-image) 共享 Gemini 5h/周配额。{}", C_DIM, C_RESET);
}

fn option_view_all_accounts() {
    clear_screen();
    println!("\n{}[*] 正在并发查询账号池中所有账号的实时配额...{}", C_DIM, C_RESET);
    let (rows, current_id, best_acc) = fetch_all_accounts_data();
    clear_screen();
    println!("{}=== 所有账号全局配额大盘 (All Accounts Overview) ==={}", C_BOLD, C_RESET);
    print_accounts_table(&rows, current_id.as_deref(), best_acc.as_ref());
    pause();
}

// [3] 切换当前活动账号
fn option_switch_account() {
    clear_screen();
    println!("\n{}{}[*] 正在读取账号池状态...{}", C_BOLD, C_CYAN, C_RESET);
    let (rows, current_id, best_acc) = fetch_all_accounts_data();
    if rows.is_empty() {
        println!("{}[-] 账号池中暂无可用账号。{}", C_RED, C_RESET);
        pause();
        return;
    }

    clear_screen();
    println!("{}=== 切换当前活动账号 (Switch Account) ==={}", C_BOLD, C_RESET);
    print_accounts_table(&rows, current_id.as_deref(), best_acc.as_ref());

    if let Some(best) = &best_acc {
        println!("提示: 输入 {}{}'auto'{} 自动切至推荐账号: {}{}{}", C_BOLD, C_MAGENTA, C_RESET, C_BOLD, best.email, C_RESET);
    }

    print!("\n>> 请输入要切换的账号序号 [0-{}] 或邮箱关键字 (按 q 取消): ", rows.len() - 1);
    let _ = io::stdout().flush();
    let mut choice = String::new();
    if io::stdin().read_line(&mut choice).is_err() {
        return;
    }
    let choice = choice.trim();
    if choice.is_empty() || choice.eq_ignore_ascii_case("q") || choice.eq_ignore_ascii_case("exit") {
        return;
    }

    let target_id = if choice.eq_ignore_ascii_case("auto") || choice.eq_ignore_ascii_case("best") {
        if let Some(b) = best_acc {
            b.id
        } else {
            println!("{}[-] 当前没有推荐账号。{}", C_RED, C_RESET);
            pause();
            return;
        }
    } else if let Ok(num) = choice.parse::<usize>() {
        if num < rows.len() {
            rows[num].acc.id.clone()
        } else {
            println!("{}[-] 序号超出范围。{}", C_RED, C_RESET);
            pause();
            return;
        }
    } else {
        if let Some(found) = rows.iter().find(|r| r.acc.email.to_lowercase().contains(&choice.to_lowercase()) || r.acc.id == choice) {
            found.acc.id.clone()
        } else {
            println!("{}[-] 未能匹配到账号: {}{}", C_RED, choice, C_RESET);
            pause();
            return;
        }
    };

    println!("\n{}[*] 正在执行切换...{}", C_DIM, C_RESET);
    match switch_account_by_id(&target_id) {
        Ok((email, count)) => {
            println!("\n{}{}[+] 账号切换成功！当前活动账号已切换为: {}{}", C_BOLD, C_GREEN, email, C_RESET);
            println!("{}凭据已更新至系统凭据管理器与本地配置。{}", C_DIM, C_RESET);
            if count > 0 {
                println!("{}{}[通过] 成功重启并恢复了 {} 个 agy 终端窗口！{}", C_BOLD, C_GREEN, count, C_RESET);
            } else {
                println!("{}[信息] 当前未检测到运行中的 agy 窗口，新账号凭据将在下次启动 agy 时生效。{}", C_DIM, C_RESET);
            }
        }
        Err(e) => {
            println!("\n{}{}[-] 切换失败: {}{}", C_BOLD, C_RED, e, C_RESET);
        }
    }
    pause();
}

// [4] 智能切至最高额度账号
fn option_auto_switch() {
    clear_screen();
    println!("\n{}{}[*] 正在智能分析各账号木桶短板余量 min(5h, weekly)...{}", C_BOLD, C_CYAN, C_RESET);
    let (_rows, _current_id, best_acc) = fetch_all_accounts_data();
    if let Some(best) = best_acc {
        println!("[*] 找到最高推荐账号: {}{}{}{}", C_BOLD, C_MAGENTA, best.email, C_RESET);
        match switch_account_by_id(&best.id) {
            Ok((email, count)) => {
                println!("\n{}{}[+] 智能切换成功！已切至最优配额账号: {}{}", C_BOLD, C_GREEN, email, C_RESET);
                if count > 0 {
                    println!("{}{}[通过] 成功重启并恢复了 {} 个 agy 终端窗口！{}", C_BOLD, C_GREEN, count, C_RESET);
                } else {
                    println!("{}[信息] 当前未检测到运行中的 agy 窗口，新账号凭据将在下次启动 agy 时生效。{}", C_DIM, C_RESET);
                }
            }
            Err(e) => {
                println!("\n{}{}[-] 自动切换失败: {}{}", C_BOLD, C_RED, e, C_RESET);
            }
        }
    } else {
        println!("\n{}[!] 当前活动账号即为最高额度账号，无需切换。{}", C_YELLOW, C_RESET);
    }
    pause();
}

// [5] 保存当前 agy 账号至账号池
fn option_save_current_account() {
    clear_screen();
    println!("\n{}{}[=== 保存当前活动 agy 账号至账号池 ===]{}", C_BOLD, C_CYAN, C_RESET);
    println!("{}从系统凭据管理器读取凭据并存入本地账号池...{}\n", C_DIM, C_RESET);

    let cred_opt = read_keyring();
    if cred_opt.is_none() {
        println!("{}[-] 未在 Windows 凭据管理器中检测到活动凭据。{}", C_RED, C_RESET);
        pause();
        return;
    }
    let cred_data = cred_opt.unwrap();
    let tok_obj = cred_data.get("token").cloned().unwrap_or(Value::Null);
    let access_tok = tok_obj.get("access_token").and_then(|t| t.as_str()).unwrap_or("").to_string();
    let refresh_tok = tok_obj.get("refresh_token").and_then(|t| t.as_str()).unwrap_or("").to_string();

    let mut email = tok_obj.get("email").and_then(|t| t.as_str()).unwrap_or("").to_string();
    let (active_email, _) = get_current_active_info();
    if email.is_empty() || email.contains("user") {
        if !active_email.is_empty() && !active_email.contains("未知") {
            email = active_email;
        }
    }

    if access_tok.is_empty() && refresh_tok.is_empty() {
        println!("{}[-] 当前凭据不完整。{}", C_RED, C_RESET);
        pause();
        return;
    }

    println!("识别到当前账号: {}{}{}{}", C_BOLD, C_WHITE, email, C_RESET);
    print!("请输入此账号的备注别名 (直接回车跳过): ");
    let _ = io::stdout().flush();
    let mut alias = String::new();
    let _ = io::stdin().read_line(&mut alias);
    let alias = alias.trim();
    let name = if !alias.is_empty() {
        Some(alias.to_string())
    } else {
        email.split('@').next().map(|s| s.to_string())
    };

    let now = Utc::now().timestamp();
    let mut idx = load_account_index();
    let acc_id = idx.accounts.iter().find(|a| a.email == email).map(|a| a.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let acc = AccountDetail {
        id: acc_id.clone(),
        email: email.clone(),
        name: name.clone(),
        token: Some(TokenInfo {
            access_token: Some(access_tok),
            refresh_token: Some(refresh_tok),
            expires_in: Some(3600),
            expiry_timestamp: Some(now + 3600),
            token_type: Some("Bearer".to_string()),
            email: Some(email.clone()),
            is_gcp_tos: Some(false),
        }),
        created_at: Some(now),
        last_used: Some(now),
        disabled: Some(false),
        proxy_disabled: Some(false),
    };
    save_account(&acc);

    if !idx.accounts.iter().any(|a| a.id == acc_id) {
        idx.accounts.push(AccountSummary {
            id: acc_id.clone(),
            email: email.clone(),
            name,
            disabled: Some(false),
            proxy_disabled: Some(false),
            created_at: Some(now),
            last_used: Some(now),
        });
    }
    idx.current_account_id = Some(acc_id);
    save_account_index(&idx);

    println!("\n{}{}[+] 成功将账号 [{}] 快照保存至账号池！{}", C_BOLD, C_GREEN, email, C_RESET);
    pause();
}

// [6] 添加新账号到账号池
#[tokio::main]
async fn run_oauth_flow() -> Result<(String, String), String> {
    let port = 43210;
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await.map_err(|e| format!("启动本地回调服务端口 43210 失败: {}", e))?;

    let redirect_uri = format!("http://127.0.0.1:{}", port);
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile%20https://www.googleapis.com/auth/cloud-platform&access_type=offline&prompt=consent",
        OAUTH_CLIENT_ID, redirect_uri
    );

    println!("\n{}[*] 正在自动打开默认浏览器进行 Google 登录授权...{}", C_GREEN, C_RESET);
    println!("{}如果浏览器未自动打开，请手动复制并在浏览器打开以下链接:{}", C_DIM, C_RESET);
    println!("{}\n", auth_url);

    let _ = Command::new("rundll32.exe").args(["url.dll,FileProtocolHandler", &auth_url]).spawn();

    println!("等待浏览器授权回调中 (等待时长约 120 秒)...");
    let mut code_found = String::new();

    let timeout = tokio::time::sleep(Duration::from_secs(120));
    tokio::pin!(timeout);

    tokio::select! {
        res = listener.accept() => {
            if let Ok((mut socket, _)) = res {
                let mut buf = [0u8; 2048];
                if let Ok(n) = socket.read(&mut buf).await {
                    let req = String::from_utf8_lossy(&buf[..n]);
                    if let Some(pos) = req.find("code=") {
                        let sub = &req[pos + 5..];
                        let end_pos = sub.find('&').or_else(|| sub.find(' ')).unwrap_or(sub.len());
                        code_found = sub[..end_pos].to_string();
                    }
                }
                let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<h1>[+] 授权成功！您可以关闭此网页并返回终端窗口了。</h1>";
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.flush().await;
            }
        }
        _ = &mut timeout => {
            return Err("等待授权超时 (120 秒)".to_string());
        }
    }

    if code_found.is_empty() {
        return Err("未能获取到有效授权码 code".to_string());
    }

    println!("\n{}[*] 正在通过授权码换取 Token...{}", C_DIM, C_RESET);
    let token_req = format!(
        "code={}&client_id={}&client_secret={}&redirect_uri={}&grant_type=authorization_code",
        code_found, OAUTH_CLIENT_ID, OAUTH_CLIENT_SECRET, redirect_uri
    );
    let res = curl_post_json(
        "https://oauth2.googleapis.com/token",
        &[("Content-Type", "application/x-www-form-urlencoded")],
        &token_req,
    )?;
    let val: Value = serde_json::from_str(&res).map_err(|e| e.to_string())?;
    let access_tok = val.get("access_token").and_then(|x| x.as_str()).ok_or("Token 响应缺失 access_token")?.to_string();
    let refresh_tok = val.get("refresh_token").and_then(|x| x.as_str()).ok_or("Token 响应缺失 refresh_token")?.to_string();

    Ok((access_tok, refresh_tok))
}

fn option_add_account() {
    clear_screen();
    println!("\n{}{}[=== 添加新账号到账号池 (Add Account) ===]{}", C_BOLD, C_CYAN, C_RESET);
    println!("--------------------------------------------------");
    println!(" [1] 浏览器一键授权登录 (自动打开 Google 登录并回调)");
    println!(" [2] 手动粘贴 Refresh Token 导入 (以 1// 开头)");
    println!(" [0] 返回主菜单");
    println!("--------------------------------------------------");

    print!(">> 请选择添加方式 [0-2]: ");
    let _ = io::stdout().flush();
    let mut choice = String::new();
    let _ = io::stdin().read_line(&mut choice);

    match choice.trim() {
        "1" => {
            match run_oauth_flow() {
                Ok((access_tok, refresh_tok)) => {
                    println!("\n{}[*] 正在查询账号信息...{}", C_DIM, C_RESET);
                    if let Ok((email, name)) = fetch_userinfo(&access_tok) {
                        let now = Utc::now().timestamp();
                        let mut idx = load_account_index();
                        let acc_id = uuid::Uuid::new_v4().to_string();
                        let acc = AccountDetail {
                            id: acc_id.clone(),
                            email: email.clone(),
                            name: Some(name.clone()),
                            token: Some(TokenInfo {
                                access_token: Some(access_tok),
                                refresh_token: Some(refresh_tok),
                                expires_in: Some(3600),
                                expiry_timestamp: Some(now + 3600),
                                token_type: Some("Bearer".to_string()),
                                email: Some(email.clone()),
                                is_gcp_tos: Some(false),
                            }),
                            created_at: Some(now),
                            last_used: Some(now),
                            disabled: Some(false),
                            proxy_disabled: Some(false),
                        };
                        save_account(&acc);

                        idx.accounts.push(AccountSummary {
                            id: acc_id,
                            email: email.clone(),
                            name: Some(name),
                            disabled: Some(false),
                            proxy_disabled: Some(false),
                            created_at: Some(now),
                            last_used: Some(now),
                        });
                        save_account_index(&idx);

                        println!("\n{}{}[+] 授权登录成功！账号 [{}] 已存入账号池！{}", C_BOLD, C_GREEN, email, C_RESET);
                    }
                }
                Err(e) => {
                    println!("\n{}{}[-] 授权流程失败: {}{}", C_BOLD, C_RED, e, C_RESET);
                }
            }
            pause();
        }
        "2" => {
            print!("\n请输入 Google OAuth Refresh Token (以 1// 开头): ");
            let _ = io::stdout().flush();
            let mut rf = String::new();
            let _ = io::stdin().read_line(&mut rf);
            let rf = rf.trim();
            if rf.is_empty() {
                println!("未输入 Token，操作已取消。");
                pause();
                return;
            }

            println!("\n{}[*] 正在验证 Token 并拉取账号信息...{}", C_DIM, C_RESET);
            match refresh_oauth_token(rf) {
                Ok((access_tok, exp)) => {
                    if let Ok((email, name)) = fetch_userinfo(&access_tok) {
                        let now = Utc::now().timestamp();
                        let mut idx = load_account_index();
                        let acc_id = uuid::Uuid::new_v4().to_string();
                        let acc = AccountDetail {
                            id: acc_id.clone(),
                            email: email.clone(),
                            name: Some(name.clone()),
                            token: Some(TokenInfo {
                                access_token: Some(access_tok),
                                refresh_token: Some(rf.to_string()),
                                expires_in: Some(exp),
                                expiry_timestamp: Some(now + exp),
                                token_type: Some("Bearer".to_string()),
                                email: Some(email.clone()),
                                is_gcp_tos: Some(false),
                            }),
                            created_at: Some(now),
                            last_used: Some(now),
                            disabled: Some(false),
                            proxy_disabled: Some(false),
                        };
                        save_account(&acc);

                        idx.accounts.push(AccountSummary {
                            id: acc_id,
                            email: email.clone(),
                            name: Some(name.clone()),
                            disabled: Some(false),
                            proxy_disabled: Some(false),
                            created_at: Some(now),
                            last_used: Some(now),
                        });
                        save_account_index(&idx);

                        println!("\n{}{}[+] 账号添加成功！邮箱: {} ({}){}", C_BOLD, C_GREEN, email, name, C_RESET);
                    } else {
                        println!("\n{}{}[-] 获取用户信息失败{}", C_BOLD, C_RED, C_RESET);
                    }
                }
                Err(e) => {
                    println!("\n{}{}[-] 验证失败: {}{}", C_BOLD, C_RED, e, C_RESET);
                }
            }
            pause();
        }
        _ => {}
    }
}

// [7] 从账号池移除账号
fn option_remove_account() {
    clear_screen();
    println!("\n{}{}[=== 从账号池移除账号 (Remove Account) ===]{}\n", C_BOLD, C_CYAN, C_RESET);

    let idx = load_account_index();
    if idx.accounts.is_empty() {
        println!("{}[-] 账号池为空。{}", C_YELLOW, C_RESET);
        pause();
        return;
    }

    for (i, a) in idx.accounts.iter().enumerate() {
        let is_cur = idx.current_account_id.as_deref() == Some(&a.id);
        let tag = if is_cur {
            format!("{}[当前*]{}", C_GREEN, C_RESET)
        } else {
            format!("{}[备用]{}", C_DIM, C_RESET)
        };
        println!(" [{}] {} {}", i, tag, a.email);
    }

    print!("\n>> 请输入要删除的账号序号 [0-{}] 或邮箱 (按 q 取消): ", idx.accounts.len() - 1);
    let _ = io::stdout().flush();
    let mut choice = String::new();
    let _ = io::stdin().read_line(&mut choice);
    let choice = choice.trim();

    if choice.is_empty() || choice.eq_ignore_ascii_case("q") || choice.eq_ignore_ascii_case("exit") {
        return;
    }

    let target_id = if let Ok(num) = choice.parse::<usize>() {
        if num < idx.accounts.len() {
            idx.accounts[num].id.clone()
        } else {
            println!("{}[-] 序号超出范围。{}", C_RED, C_RESET);
            pause();
            return;
        }
    } else {
        if let Some(a) = idx.accounts.iter().find(|x| x.email.eq_ignore_ascii_case(choice) || x.id == choice) {
            a.id.clone()
        } else {
            println!("{}[-] 未匹配到账号。{}", C_RED, C_RESET);
            pause();
            return;
        }
    };

    let mut new_idx = idx.clone();
    let mut removed_email = String::new();
    new_idx.accounts.retain(|a| {
        if a.id == target_id {
            removed_email = a.email.clone();
            false
        } else {
            true
        }
    });

    if new_idx.current_account_id.as_deref() == Some(&target_id) {
        new_idx.current_account_id = None;
    }
    save_account_index(&new_idx);

    let acc_file = get_accounts_dir().join(format!("{}.json", target_id));
    let _ = fs::remove_file(acc_file);

    println!("\n{}{}[+] 成功移除账号: {}{}", C_BOLD, C_GREEN, removed_email, C_RESET);
    pause();
}

// [8] 强制刷新当前账号凭据
fn option_force_refresh() {
    clear_screen();
    println!("\n{}[*] 正在强制刷新当前活动账号的 OAuth Token...{}", C_DIM, C_RESET);
    match get_valid_active_token(true) {
        Ok(_) => {
            println!("{}{}[+] 成功刷新 Token 并同步至系统凭据管理器！{}", C_BOLD, C_GREEN, C_RESET);
        }
        Err(e) => {
            println!("{}{}[-] 刷新失败: {}{}", C_BOLD, C_RED, e, C_RESET);
        }
    }
    pause();
}

// -----------------------------------------------------------------------------
// Interactive Console Loop
// -----------------------------------------------------------------------------
// [9] 自动重启所有运行中的 agy 终端会话
fn option_reload_all_sessions() {
    clear_screen();
    println!("\n{}{}[*] 正在扫描并热重载所有运行中的 agy 终端会话...{}", C_BOLD, C_CYAN, C_RESET);
    let count = reload_all_active_agy_sessions();
    if count > 0 {
        println!("\n{}{}[通过] 成功重启并恢复了 {} 个 agy 终端窗口！{}", C_BOLD, C_GREEN, count, C_RESET);
    } else {
        println!("\n{}[信息] 当前未检测到正在运行的 agy 终端窗口。{}", C_DIM, C_RESET);
    }
    pause();
}

fn run_cli_menu() {
    init_console();
    loop {
        clear_screen();
        let (email, tier) = get_current_active_info();
        println!("{}======================================================================{}", C_BOLD, C_RESET);
        println!("{} >>> Antigravity (AGY) 多账号与配额管理中心 v2.0.0 (Rust) <<<{}", C_BOLD, C_RESET);
        println!("{}======================================================================{}", C_BOLD, C_RESET);
        println!(" 当前活动账号: {}{}{}  |  套餐类型: {}{}{}", C_WHITE, email, C_RESET, C_YELLOW, tier, C_RESET);
        println!(" 系统本地时间: {}{}{}", C_DIM, Local::now().format("%Y-%m-%d %H:%M:%S"), C_RESET);
        println!("{}", "-".repeat(70));
        println!(" {}[1]{} 查看当前账号额度详情 (Detailed Quota)", C_CYAN, C_RESET);
        println!(" {}[2]{} 查看所有账号全局大盘 (All Accounts Overview)", C_CYAN, C_RESET);
        println!(" {}[3]{} 切换当前活动账号 (Switch Account - 交互选择 / 序号 / 邮箱)", C_CYAN, C_RESET);
        println!(" {}[4]{} 智能切至最高额度账号 (Auto-Switch to Best Quota Account)", C_CYAN, C_RESET);
        println!(" {}[5]{} 保存当前 agy 账号至账号池 (Save Active agy Account)", C_CYAN, C_RESET);
        println!(" {}[6]{} 添加新账号到账号池 (Add New Account - 浏览器授权 / Token)", C_CYAN, C_RESET);
        println!(" {}[7]{} 从账号池移除账号 (Remove Account)", C_CYAN, C_RESET);
        println!(" {}[8]{} 强制刷新当前账号凭据 (Refresh Token)", C_CYAN, C_RESET);
        println!(" {}[9]{} 自动重启所有运行中的 agy 终端会话 (Reload All Sessions)", C_CYAN, C_RESET);
        println!(" {}[0]{} 退出程序 (Exit)", C_RED, C_RESET);
        println!("{}======================================================================{}", C_BOLD, C_RESET);

        print!(">> 请输入选项 {}[0-9]{}: ", C_BOLD, C_RESET);
        let _ = io::stdout().flush();
        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            break;
        }

        match choice.trim() {
            "1" => option_view_current_quota(),
            "2" => option_view_all_accounts(),
            "3" => option_switch_account(),
            "4" => option_auto_switch(),
            "5" => option_save_current_account(),
            "6" => option_add_account(),
            "7" => option_remove_account(),
            "8" => option_force_refresh(),
            "9" => option_reload_all_sessions(),
            "0" | "q" | "quit" | "exit" => {
                println!("\n{}[+] 感谢使用 Antigravity 配额中心，再见！{}\n", C_GREEN, C_RESET);
                break;
            }
            _ => {
                println!("{}[!] 无效选项，请重新输入。{}", C_RED, C_RESET);
                std::thread::sleep(Duration::from_millis(600));
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Main Entry
// -----------------------------------------------------------------------------
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let subcommand = args.get(1).map(|s| s.to_lowercase()).unwrap_or_default();

    // 内部工作进程注入：直接在后台附加到指定控制台并执行注入，不唤起新窗口
    if subcommand == "__worker_inject" {
        if let (Some(pid_str), Some(action)) = (args.get(2), args.get(3)) {
            if let Ok(target_pid) = pid_str.parse::<u32>() {
                let code = unsafe { run_worker_inject(target_pid, action) };
                std::process::exit(code);
            }
        }
        std::process::exit(1);
    }

    // If double-clicked from desktop without subcommands and not yet inside Windows Terminal,
    // relaunch via Windows Terminal (wt.exe) for gorgeous typography & GPU font rendering!
    if subcommand.is_empty() && std::env::var("WT_SESSION").is_err() && !args.iter().any(|a| a == "--no-wt") {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Ok(_) = Command::new("wt.exe")
                .args([
                    "--title", "Antigravity (AGY) 多账号与配额管理中心",
                    exe_path.to_str().unwrap_or(""),
                    "--no-wt",
                ])
                .spawn()
            {
                return;
            }
        }
    }

    init_console();

    if !subcommand.is_empty() && subcommand != "--no-wt" {
        match subcommand.as_str() {
            "usage" | "status" | "overview" | "panel" => {
                println!("\n[*] 正在查询各账号实时配额...");
                let (rows, current_id, best_acc) = fetch_all_accounts_data();
                print_accounts_table(&rows, current_id.as_deref(), best_acc.as_ref());
                return;
            }
            "accounts" | "list" => {
                let idx = load_account_index();
                println!("\n已注册账号列表 (共 {} 个):", idx.accounts.len());
                for (i, a) in idx.accounts.iter().enumerate() {
                    let is_curr = idx.current_account_id.as_deref() == Some(&a.id);
                    let mark = if is_curr { "[当前*]" } else { "       " };
                    println!("  {} [{}] {} ({})", mark, i + 1, a.email, a.name.as_deref().unwrap_or("-"));
                }
                return;
            }
            "auto" | "best" => {
                println!("\n[*] 正在计算各账号有效短板容量...");
                let (_rows, _, best_acc) = fetch_all_accounts_data();
                if let Some(best) = best_acc {
                    match switch_account_by_id(&best.id) {
                        Ok((email, count)) => {
                            println!("[+] 智能切换成功！已切至最优配额账号: {}", email);
                            if count > 0 {
                                println!("[通过] 成功重启并恢复了 {} 个 agy 终端窗口！", count);
                            }
                        }
                        Err(e) => println!("[-] 智能切换失败: {}", e),
                    }
                } else {
                    println!("[!] 未检测到可用推荐账号。");
                }
                return;
            }
            "reload" | "restart" => {
                let count = reload_all_active_agy_sessions();
                if count > 0 {
                    println!("[通过] 成功重启并恢复了 {} 个 agy 终端窗口！", count);
                } else {
                    println!("[信息] 当前未检测到正在运行的 agy 窗口。");
                }
                return;
            }
            "switch" => {
                let idx = load_account_index();
                if let Some(target_arg) = args.get(2) {
                    if let Ok(n) = target_arg.parse::<usize>() {
                        if n >= 1 && n <= idx.accounts.len() {
                            let target_id = &idx.accounts[n - 1].id;
                            match switch_account_by_id(target_id) {
                                Ok((email, count)) => {
                                    println!("[+] 切换成功！当前活动账号已切为: {}", email);
                                    if count > 0 {
                                        println!("[通过] 成功重启并恢复了 {} 个 agy 终端窗口！", count);
                                    }
                                }
                                Err(e) => println!("[-] 切换失败: {}", e),
                            }
                            return;
                        }
                    }
                    if let Some(acc) = idx.accounts.iter().find(|a| a.email.eq_ignore_ascii_case(target_arg) || a.id == *target_arg) {
                        match switch_account_by_id(&acc.id) {
                            Ok((email, count)) => {
                                println!("[+] 切换成功！当前活动账号已切为: {}", email);
                                if count > 0 {
                                    println!("[通过] 成功重启并恢复了 {} 个 agy 终端窗口！", count);
                                }
                            }
                            Err(e) => println!("[-] 切换失败: {}", e),
                        }
                        return;
                    }
                    println!("[-] 未找到匹配的账号: {}", target_arg);
                    return;
                }
                println!("\n请选择要切换的目标账号序号:");
                for (i, a) in idx.accounts.iter().enumerate() {
                    println!("  [{}] {}", i + 1, a.email);
                }
                print!(">> 输入序号 [1-{}]: ", idx.accounts.len());
                let _ = io::stdout().flush();
                let mut num_in = String::new();
                if io::stdin().read_line(&mut num_in).is_ok() {
                    if let Ok(n) = num_in.trim().parse::<usize>() {
                        if n >= 1 && n <= idx.accounts.len() {
                            let target_id = &idx.accounts[n - 1].id;
                            match switch_account_by_id(target_id) {
                                Ok((email, count)) => {
                                    println!("[+] 切换成功！当前活动账号已切为: {}", email);
                                    if count > 0 {
                                        println!("[通过] 成功重启并恢复了 {} 个 agy 终端窗口！", count);
                                    }
                                }
                                Err(e) => println!("[-] 切换失败: {}", e),
                            }
                        }
                    }
                }
                return;
            }
            "save" => {
                option_save_current_account();
                return;
            }
            "--help" | "-h" | "help" => {
                println!("Antigravity (AGY) 多账号与配额管理中心 v2.0.0 (Rust Console)");
                println!("用法:");
                println!("  AGY多账号配额中心.exe            # 默认打开交互式黑窗口控制台菜单");
                println!("  AGY多账号配额中心.exe usage      # 直接输出当前各账号配额对比表");
                println!("  AGY多账号配额中心.exe accounts   # 列出所有账号");
                println!("  AGY多账号配额中心.exe switch <序号/邮箱> # 切换活动账号并自动热重载终端");
                println!("  AGY多账号配额中心.exe auto       # 自动切至最高额度账号并自动热重载终端");
                println!("  AGY多账号配额中心.exe reload     # 手动触发所有运行中 agy 终端会话的热重载");
                println!("  AGY多账号配额中心.exe save       # 保存当前账号快照");
                return;
            }
            _ => {}
        }
    }

    // Default: Run interactive console menu
    run_cli_menu();
}
