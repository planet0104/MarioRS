//! 持久化工具模块
//!
//! 提供轻量级的二进制序列化/反序列化工具，用于游戏配置和状态的存取。
//! 完全跨平台兼容，使用小端序格式确保不同平台间数据一致。

use std::io::{Read, Write};

// 重导出 Cursor 供外部模块使用
pub use std::io::Cursor;

// ============================================================================
// 基础类型读写
// ============================================================================

/// 写入 u8
pub fn write_u8<W: Write>(w: &mut W, v: u8) -> std::io::Result<()> {
    w.write_all(&[v])
}

/// 读取 u8
pub fn read_u8<R: Read>(r: &mut R) -> std::io::Result<u8> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}

/// 写入 u16（小端序）
pub fn write_u16_le<W: Write>(w: &mut W, v: u16) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

/// 读取 u16（小端序）
pub fn read_u16_le<R: Read>(r: &mut R) -> std::io::Result<u16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}

/// 写入 i16（小端序）
pub fn write_i16_le<W: Write>(w: &mut W, v: i16) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

/// 读取 i16（小端序）
pub fn read_i16_le<R: Read>(r: &mut R) -> std::io::Result<i16> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(i16::from_le_bytes(b))
}

/// 写入 i32（小端序）
pub fn write_i32_le<W: Write>(w: &mut W, v: i32) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

/// 读取 i32（小端序）
pub fn read_i32_le<R: Read>(r: &mut R) -> std::io::Result<i32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}

/// 写入布尔值（1 字节）
pub fn write_bool<W: Write>(w: &mut W, v: bool) -> std::io::Result<()> {
    write_u8(w, if v { 1 } else { 0 })
}

/// 读取布尔值（1 字节）
pub fn read_bool<R: Read>(r: &mut R) -> std::io::Result<bool> {
    Ok(read_u8(r)? != 0)
}

// ============================================================================
// 数组类型读写（用于游戏数据的 2 玩家数组）
// ============================================================================

/// 写入 [i16; 2] 数组
pub fn write_i16_pair<W: Write>(w: &mut W, arr: [i16; 2]) -> std::io::Result<()> {
    write_i16_le(w, arr[0])?;
    write_i16_le(w, arr[1])
}

/// 读取 [i16; 2] 数组
pub fn read_i16_pair<R: Read>(r: &mut R) -> std::io::Result<[i16; 2]> {
    Ok([read_i16_le(r)?, read_i16_le(r)?])
}

/// 写入 [i32; 2] 数组
pub fn write_i32_pair<W: Write>(w: &mut W, arr: [i32; 2]) -> std::io::Result<()> {
    write_i32_le(w, arr[0])?;
    write_i32_le(w, arr[1])
}

/// 读取 [i32; 2] 数组
pub fn read_i32_pair<R: Read>(r: &mut R) -> std::io::Result<[i32; 2]> {
    Ok([read_i32_le(r)?, read_i32_le(r)?])
}

/// 写入 [u8; 2] 数组
pub fn write_u8_pair<W: Write>(w: &mut W, arr: [u8; 2]) -> std::io::Result<()> {
    write_u8(w, arr[0])?;
    write_u8(w, arr[1])
}

/// 读取 [u8; 2] 数组
pub fn read_u8_pair<R: Read>(r: &mut R) -> std::io::Result<[u8; 2]> {
    Ok([read_u8(r)?, read_u8(r)?])
}

// ============================================================================
// 字符串读写
// ============================================================================

/// 写入 UTF-8 字符串（u16 长度前缀）
pub fn write_string<W: Write>(w: &mut W, s: &str) -> std::io::Result<()> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len > u16::MAX as usize {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "string too long"));
    }
    write_u16_le(w, len as u16)?;
    w.write_all(bytes)
}

/// 读取 UTF-8 字符串（u16 长度前缀）
pub fn read_string<R: Read>(r: &mut R) -> std::io::Result<String> {
    let len = read_u16_le(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
