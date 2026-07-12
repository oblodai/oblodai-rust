//! Опциональное, безопасное логирование SDK — БЕЗ внешних зависимостей.
//!
//! Логирование выключено по умолчанию. Включить можно двумя способами:
//! 1. Программно — передать колбэк в [`crate::Config::logger`].
//! 2. Через окружение — задать `OBLODAI_LOG=debug|info|warn|error`; тогда SDK установит
//!    встроенный логгер в stderr (`eprintln!`), отфильтрованный по уровню.
//!
//! БЕЗОПАСНОСТЬ: SDK НИКОГДА не логирует секрет, значение подписи, секрет вебхука или тела
//! запросов/ответов. Логируются только method/path/status/ms/attempt/delay/код ошибки (public_id
//! несекретен и допустим).

use std::sync::Arc;

/// Уровень сообщения лога.
///
/// Порядок вариантов задаёт отношение «важности» (`Debug < Info < Warn < Error`), что используется
/// для фильтрации по минимальному уровню.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Подробная трассировка (старт запроса, ответ, успех проверки вебхука).
    Debug,
    /// Информационные сообщения.
    Info,
    /// Предупреждения (повтор, финальная ошибка, отказ проверки вебхука).
    Warn,
    /// Ошибки.
    Error,
}

impl LogLevel {
    /// Текстовая метка уровня для встроенного stderr-логгера.
    fn label(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// Разбирает значение `OBLODAI_LOG`. Регистр не важен; неизвестное значение — `None`.
    fn parse(s: &str) -> Option<LogLevel> {
        match s.trim().to_ascii_lowercase().as_str() {
            "debug" | "trace" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" | "warning" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// Колбэк-логгер: вызывается с уровнем и уже отформатированным сообщением.
///
/// Тип `Send + Sync`, поэтому клиент остаётся потокобезопасным. Клонируется дёшево (это `Arc`).
pub type Logger = Arc<dyn Fn(LogLevel, &str) + Send + Sync>;

/// Имя переменной окружения для включения встроенного логгера.
pub const ENV_LOG: &str = "OBLODAI_LOG";

/// Строит встроенный stderr-логгер из `OBLODAI_LOG`, если переменная задана валидным уровнем.
///
/// Возвращает `None`, если переменная не задана/пуста/содержит неизвестный уровень — тогда
/// логирование остаётся выключенным. Уровень читается один раз (замыкается в колбэк).
pub(crate) fn env_logger() -> Option<Logger> {
    let raw = std::env::var(ENV_LOG).ok()?;
    let min = LogLevel::parse(&raw)?;
    let logger: Logger = Arc::new(move |level: LogLevel, msg: &str| {
        if level >= min {
            eprintln!("[oblodai {}] {}", level.label(), msg);
        }
    });
    Some(logger)
}

/// Логирует через встроенный env-логгер, если `OBLODAI_LOG` задан. Используется в местах без клиента
/// (проверка вебхуков). No-op, если переменная не задана.
pub(crate) fn log_env(level: LogLevel, msg: &str) {
    if let Some(logger) = env_logger() {
        logger(level, msg);
    }
}
