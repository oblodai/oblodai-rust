//! Небольшой источник случайности поверх `getrandom` (уже в дереве зависимостей через reqwest/rustls).
//!
//! Используется для ключей идемпотентности (`Idempotency-Key`) и джиттера повторов. Тяжёлых
//! зависимостей (`rand`/`uuid`) не тянем.

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

/// UUID v4 (RFC 4122) в каноничной строковой форме — значение заголовка `Idempotency-Key`.
pub(crate) fn uuid4() -> String {
    let mut buf = [0u8; 16];
    fill(&mut buf);
    buf[6] = (buf[6] & 0x0f) | 0x40; // версия 4
    buf[8] = (buf[8] & 0x3f) | 0x80; // вариант RFC 4122
    let h = hex::encode(buf);
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// Случайное число в диапазоне `[0.0, 1.0)`.
pub(crate) fn unit_f64() -> f64 {
    let mut buf = [0u8; 8];
    fill(&mut buf);
    let v = u64::from_le_bytes(buf);
    // 53 значимых бита мантиссы f64.
    (v >> 11) as f64 / ((1u64 << 53) as f64)
}
