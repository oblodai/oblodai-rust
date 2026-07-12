//! Небольшой источник случайности поверх `getrandom` (уже в дереве зависимостей через reqwest/rustls).
//!
//! Используется для идемпотентных ключей (`order_id`) и джиттера повторов. Тяжёлых зависимостей
//! (`rand`/`uuid`) не тянем.

/// Заполняет буфер случайными байтами из ОС-RNG. При крайне редком сбое ОС-RNG использует
/// запасной xorshift-источник (наносекунды + адрес буфера), чтобы никогда не паниковать —
/// best-effort уникальность.
fn fill(buf: &mut [u8]) {
    if getrandom::getrandom(buf).is_ok() {
        return;
    }
    let mut x = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ (buf.as_ptr() as u64);
    x |= 1;
    for b in buf.iter_mut() {
        // xorshift64
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = (x & 0xff) as u8;
    }
}

/// 16 случайных байт в hex (32 символа) — годится как идемпотентный ключ.
pub(crate) fn hex16() -> String {
    let mut buf = [0u8; 16];
    fill(&mut buf);
    hex::encode(buf)
}

/// Случайное число в диапазоне `[0.0, 1.0)`.
pub(crate) fn unit_f64() -> f64 {
    let mut buf = [0u8; 8];
    fill(&mut buf);
    let v = u64::from_le_bytes(buf);
    // 53 значимых бита мантиссы f64.
    (v >> 11) as f64 / ((1u64 << 53) as f64)
}
