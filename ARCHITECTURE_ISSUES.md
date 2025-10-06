# Архитектурный анализ и выявленные проблемы

## 🔴 Критические проблемы

### 1. DropOldest strategy не полностью реализована

**Файл:** `core/src/backpressure.rs:137`

**Проблема:**
```rust
BackpressureStrategy::DropOldest => {
    // TODO: Implement proper DropOldest with VecDeque
    warn!("DropOldest strategy not fully implemented, using Block");
    // Fallback to Block strategy
}
```

**Влияние:**
- Пользователь выбирает `DropOldest`, но получает `Block` поведение
- Нет предупреждения в документации
- Может привести к блокировке вместо дропа

**Решение:**
Реализовать полноценную поддержку `DropOldest` с использованием `VecDeque`:
```rust
use std::collections::VecDeque;

struct BackpressureController {
    queue: Arc<RwLock<VecDeque<Event>>>,
    max_size: usize,
    // ...
}

async fn send(&self, event: Event) {
    let mut queue = self.queue.write().await;
    if queue.len() >= self.max_size {
        queue.pop_front(); // Drop oldest
    }
    queue.push_back(event);
}
```

---

## 🟡 Предупреждения (Warnings)

### 2. Неиспользуемые поля в SubscriptionMeta

**Файлы:**
- `transport/nats/src/subscriber.rs:18`
- `transport/redis/src/subscriber.rs:19`
- `transport/rabbitmq/src/subscriber.rs:19`
- `transport/websocket/src/subscriber.rs:19`

**Проблема:**
```rust
struct SubscriptionMeta {
    task_handle: tokio::task::JoinHandle<()>,
    event_types: Vec<String>,  // ← Никогда не используется
}
```

**Влияние:**
- Тратится память на хранение неиспользуемых данных
- Код выглядит незавершенным

**Решение:**
Либо использовать поле (например, для фильтрации), либо удалить:
```rust
// Вариант 1: Использовать для логирования/метрик
impl SubscriptionMeta {
    fn event_types(&self) -> &[String] {
        &self.event_types
    }
}

// Вариант 2: Удалить
struct SubscriptionMeta {
    task_handle: tokio::task::JoinHandle<()>,
    // event_types убрано
}
```

### 3. Неиспользуемые поля error в DlqEntry

**Файлы:**
- `transport/webhook/src/client.rs:19`
- `transport/websocket/src/transport.rs:26`

**Проблема:**
```rust
struct DlqEntry {
    event: Event,
    error: String,  // ← Никогда не читается
}
```

**Влияние:**
- При просмотре DLQ нет информации о причине ошибки
- Невозможно отладить проблемы

**Решение:**
Использовать поле при логировании/обработке DLQ:
```rust
async fn consume_dlq(&self, callback: EventCallback) {
    for entry in &self.dlq {
        log::error!("DLQ entry failed with: {}", entry.error);
        callback(entry.event.clone()).await?;
    }
}
```

### 4. Неиспользуемые импорты

**Файлы:**
- `transport/websocket/src/transport.rs:8` - `RwLock`
- `transport/webhook/src/subscriber.rs:10` - `BackpressureConfig`, `BackpressureStrategy`
- `transport/nats/src/subscriber.rs:11` - `BackpressureStrategy`

**Решение:**
```bash
cargo clippy --fix --allow-dirty
```

### 5. Неиспользуемый параметр event_type

**Файл:** `core/src/event.rs:66`

**Проблема:**
```rust
pub fn with_metadata(event_type: impl Into<String>, payload: Value, metadata: EventMetadata) -> Self {
    // event_type не используется!
}
```

**Влияние:**
- API вводит в заблуждение
- Метод не делает то, что обещает

**Решение:**
Либо использовать параметр:
```rust
pub fn with_metadata(event_type: impl Into<String>, payload: Value, metadata: EventMetadata) -> Self {
    Self {
        metadata: EventMetadata {
            event_type: event_type.into(),
            ..metadata
        },
        payload,
    }
}
```

Либо удалить параметр и переименовать метод.

---

## 🟢 Архитектурные улучшения (не критично, но желательно)

### 6. Несогласованность API для Webhook

**Проблема:**
WebhookSubscriber имеет `subscribe()` метод из трейта `Subscriber`, но по сути Webhook - это publish-only транспорт.

**Текущая реализация:**
```rust
impl Subscriber for WebhookSubscriber {
    async fn subscribe(&mut self, ...) -> Result<SubscriptionHandle, ...> {
        // Сохраняет callback, но callback редко вызывается
        // Основной use case: register_endpoint() + publish()
    }
}
```

**Проблема с API:**
- Пользователь может подумать, что `subscribe()` работает как в других транспортах
- Реальный API: `register_endpoint()` для webhook'ов
- Два способа делать почти одно и то же

**Решение:**

**Вариант А:** Разделить трейты
```rust
pub trait Publisher {
    async fn publish(&self, event: Event) -> Result<(), WatchtowerError>;
}

pub trait Subscriber: Publisher {
    async fn subscribe(...) -> Result<SubscriptionHandle, ...>;
    async fn unsubscribe(...) -> Result<(), ...>;
}

// WebhookSubscriber реализует только Publisher
impl Publisher for WebhookSubscriber { ... }

// Другие транспорты реализуют Subscriber (который extends Publisher)
impl Subscriber for NatsSubscriber { ... }
```

**Вариант Б:** Документировать текущее поведение
Добавить в документацию четкое объяснение:
```rust
/// Webhook is a publish-only transport.
/// Use `register_endpoint()` to configure webhook URLs,
/// then `publish()` to deliver events.
/// The `subscribe()` method is available but primarily for
/// internal callbacks, not for receiving external events.
```

### 7. Отсутствие graceful shutdown

**Проблема:**
При вызове `unsubscribe()` задачи просто дропаются. Нет гарантии, что все сообщения обработаны.

**Текущая реализация:**
```rust
async fn unsubscribe(&mut self, handle: &SubscriptionHandle) {
    let mut subs = self.subscriptions.write().await;
    subs.remove(&handle.id);
    // JoinHandle дропается → задача отменяется
}
```

**Решение:**
```rust
async fn unsubscribe(&mut self, handle: &SubscriptionHandle) -> Result<(), WatchtowerError> {
    let mut subs = self.subscriptions.write().await;

    if let Some(meta) = subs.remove(&handle.id) {
        // Graceful shutdown: дождаться завершения задачи
        meta.task_handle.await
            .map_err(|e| WatchtowerError::InternalError(format!("Task join error: {}", e)))?;
    }

    Ok(())
}
```

### 8. Потенциальная проблема с backpressure receive/send

**Проблема:**
В `publish()` вызываются `backpressure.send()` и сразу `backpressure.receive()`:

```rust
async fn publish(&self, event: Event) -> Result<(), WatchtowerError> {
    self.backpressure.send(event).await?;

    if let Some(queued_event) = self.backpressure.receive().await {
        // Обработка
    }
}
```

**Вопрос:**
- Что если между `send()` и `receive()` другой поток вызовет `receive()`?
- Может ли это привести к потере сообщений?

**Текущая защита:**
`receive()` использует `RwLock<Receiver>`, что защищает от одновременного доступа.

**Потенциальная проблема:**
Если два потока вызывают `publish()` одновременно:
1. Thread A: `send(event1)` → очередь: [event1]
2. Thread B: `send(event2)` → очередь: [event1, event2]
3. Thread B: `receive()` → получает event1 (!)
4. Thread A: `receive()` → получает event2 (!)

Events обработаны в другом порядке.

**Решение (если это проблема):**
Использовать семафор или мьютекс вокруг `send + receive`:
```rust
pub async fn send_and_process<F>(&self, event: Event, processor: F)
where F: FnOnce(Event) -> Result<(), WatchtowerError>
{
    let _guard = self.send_mutex.lock().await;

    self.sender.send(event).await?;
    if let Some(event) = self.receiver.write().await.recv().await {
        processor(event)?;
    }
}
```

### 9. Отсутствие метрик и наблюдаемости

**Проблема:**
Нет встроенной поддержки для:
- Prometheus метрик
- OpenTelemetry трейсинга
- Structured logging

**Решение:**
Добавить опциональные features:
```toml
[features]
default = []
metrics = ["prometheus"]
tracing = ["opentelemetry"]
```

```rust
#[cfg(feature = "metrics")]
fn record_publish_metric(&self) {
    metrics::counter!("watchtower_events_published_total", 1);
}
```

### 10. Circuit breaker не персистентный

**Проблема:**
При перезапуске приложения состояние circuit breaker теряется. Сервис, который был в состоянии OPEN, снова начнет получать запросы.

**Решение:**
Добавить опциональную персистенцию состояния:
```rust
pub struct CircuitBreakerConfig {
    // ...
    pub state_persistence: Option<Box<dyn StatePersistence>>,
}

pub trait StatePersistence {
    async fn save_state(&self, state: CircuitState) -> Result<(), Error>;
    async fn load_state(&self) -> Result<Option<CircuitState>, Error>;
}
```

---

## 🔵 Несоответствия и странности

### 11. Разные названия полей в конфигах

**NATS:**
```rust
max_reconnect_attempts: u32
reconnect_delay_seconds: u64
```

**WebSocket:**
```rust
retry_attempts: u32  // ← разное название!
retry_delay_seconds: u64
```

**Решение:**
Унифицировать naming:
```rust
// Везде использовать:
retry_attempts: u32
retry_delay_seconds: u64
```

### 12. Несогласованность в обработке ошибок DLQ

**RabbitMQ:** DLX настраивается в конфиге
**Redis:** DLQ stream создается автоматически
**Webhook/WebSocket:** DLQ в памяти (теряется при перезапуске)

**Решение:**
Унифицировать подход:
```rust
pub enum DlqBackend {
    InMemory,
    Persistent { url: String },
    Custom(Box<dyn DlqStorage>),
}
```

---

## 📊 Итоговая оценка

### Критичность проблем:

| Уровень | Количество | Проблемы |
|---------|-----------|----------|
| 🔴 Критично | 1 | DropOldest не работает |
| 🟡 Важно | 5 | Неиспользуемые поля, импорты, параметры |
| 🟢 Желательно | 6 | API несогласованность, graceful shutdown, метрики |

### Приоритеты исправления:

1. **Немедленно:** Реализовать DropOldest или удалить из API
2. **Скоро:** Убрать dead code (неиспользуемые поля)
3. **Планово:** Унифицировать API и naming
4. **Будущее:** Добавить метрики и персистенцию

---

## ✅ Что сделано хорошо

1. ✅ Асинхронная архитектура с tokio
2. ✅ Использование трейтов для абстракции
3. ✅ Circuit breaker реализован
4. ✅ Backpressure поддерживается
5. ✅ Comprehensive тесты и примеры
6. ✅ Хорошая документация
7. ✅ Нет unwrap() в production коде
8. ✅ Использование Result для обработки ошибок

---

## 🎯 Рекомендации

### Краткосрочные (1-2 недели):
1. Реализовать DropOldest стратегию
2. Убрать dead code (cargo clippy --fix)
3. Исправить неиспользуемый параметр в with_metadata()
4. Добавить использование error поля в DlqEntry

### Среднесрочные (1 месяц):
1. Унифицировать naming в конфигах
2. Разделить Publisher/Subscriber трейты
3. Реализовать graceful shutdown
4. Добавить метрики (Prometheus)

### Долгосрочные (3 месяца):
1. Унифицировать DLQ backends
2. Добавить OpenTelemetry трacing
3. Персистентный circuit breaker state
4. Performance benchmarks
5. Load testing

---

## 🔧 Команды для исправления

```bash
# Убрать warnings
cargo clippy --fix --allow-dirty --all-targets

# Проверить все проблемы
cargo clippy --all-targets --all-features -- -D warnings

# Запустить тесты
make test-all

# Проверить документацию
cargo doc --no-deps --all-features
```
