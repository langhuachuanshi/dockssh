//! 终端 (PTY) 模块：docker exec -it 的双向透传。
//!
//! 实现思路：对 SSH channel 请求 pty + exec，
//! - 收到前端输入 → 写 channel.data()
//! - channel 数据 → emit 给前端 xterm
//! - resize 事件 → channel.window_change()
//!
//! 第一版提供 per-session 的 ExecPty 句柄管理。
