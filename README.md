# Описание `std::sync::mpsc` в Rust

## Что такое `mpsc`

**`mpsc`** — **Multi-Producer, Single-Consumer** — канал, в который **много отправителей** могут писать, а **один получатель** читает.

```rust
use std::sync::mpsc;

let (tr, rs) = mpsc::channel::<i32>();
```

- **`tr`** — `Sender<T>` (отправитель, **клонируемый**).
- **`rs`** — `Receiver<T>` (получатель, **НЕ клонируемый**).

Это **встроенный** канал в стандартной библиотеке Rust.

## Особенности `std::sync::mpsc`

### 1. **Multi-producer** — много отправителей

```rust
let (tr, rs) = mpsc::channel::<i32>();

let tr1 = tr.clone();   // ✅ Sender клонируется
let tr2 = tr.clone();
let tr3 = tr.clone();
```

- **`Sender<T>` реализует `Clone`**.
- Можно **много** отправителей.
- Каждый клон **независим**.

### 2. **Single-consumer** — один получатель

```rust
let rs2 = rs.clone();   // ❌ Receiver НЕ клонируется
```

- **`Receiver<T>` НЕ реализует `Clone`**.
- Только **один** получатель.
- Для нескольких — используйте `Arc<Mutex<Receiver>>` или **другой** канал.

### 3. **Передача по значению**

```rust
tr.send(42).unwrap();   // ← значение ПЕРЕМЕЩАЕТСЯ
```

- Значение **передаётся** (перемещается) в канал.
- Для `Copy`-типов (`i32`, `bool`) — **копируется**.

### 4. **Трейты `Send` и `Sync`**

| Тип | `Send` | `Sync` |
|---|---|---|
| `Sender<T>` | ✅ Да | ✅ Да |
| `Receiver<T>` | ✅ Да | ❌ **Нет** |

**Что это значит:**

- **`Sender`** — можно **перемещать** и **делить** между потоками.
- **`Receiver`** — можно **перемещать**, но **нельзя делить** (не `Sync`).
- Один получатель → **нет гонок** на приёме.

### 5. **Очередь неограниченная**

```rust
let (tr, rs) = mpsc::channel::<i32>();   // ← НЕОГРАНИЧЕННАЯ очередь
```

**Ключевая проблема:** `std::sync::mpsc` использует **неограниченную** очередь.

```rust
// Быстрый отправитель
for i in 0..1_000_000 {
    tr.send(i).unwrap();   // ← не блокируется, копит в памяти
}

// Медленный получатель
for r in rs {
    thread::sleep(Duration::from_millis(1));
    println!("{}", r);
}
```

**Проблема:** если получатель **медленнее** — **память растёт** → **OOM**.

## Разбор примера

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tr, rs) = mpsc::channel::<i32>();
    let mut hands = vec![];

    for i in 0..100 {
        hands.push(
            thread::spawn({
                let tr = tr.clone();   // ← клон отправителя
                move || {
                    tr.send(i).unwrap();   // ← отправка
                }
            })
        );
    }

    drop(tr);   // ← уничтожаем оригинальный отправитель

    for h in hands {
        h.join().unwrap();
    }

    for r in rs {
        println!("From thread N: {r}");
    }
}
```

### Что происходит

1. **Создан канал** `(tr, rs)`.
2. **100 потоков** — каждый клонирует `tr` и отправляет `i`.
3. **`drop(tr)`** — уничтожает **оригинальный** отправитель.
4. **`join`** — ждём завершения всех потоков.
5. **`for r in rs`** — читаем, пока канал **не закрыт**.

### Почему `drop(tr)` **обязателен**

`for r in rs` завершается, когда **ВСЕ** отправители **уничтожены**:

- каждый поток клонировал `tr` → после `send` его клон **уничтожается**;
- **оригинальный** `tr` в `main` **остаётся**;
- **без** `drop(tr)` → канал **не закрыт** → `for r in rs` **зависнет**.

## Полный разбор трейтов

### `Sender<T>`

```rust
pub struct Sender<T> { /* ... */ }

impl<T> Clone for Sender<T> { ... }   // ✅ Clone
unsafe impl<T: Send> Send for Sender<T> { }
unsafe impl<T: Send> Sync for Sender<T> { }
```

- **`Clone`** — да.
- **`Send`** — да (если `T: Send`).
- **`Sync`** — да (если `T: Send`).

### `Receiver<T>`

```rust
pub struct Receiver<T> { /* ... */ }

// НЕТ impl Clone
unsafe impl<T: Send> Send for Receiver<T> { }
// НЕТ unsafe impl Sync
```

- **`Clone`** — **нет**.
- **`Send`** — да.
- **`Sync`** — **нет**.

**`!Sync`** означает: **нельзя** делить `&Receiver` между потоками. Только **один** поток может **читать**.

## Проблемы `std::sync::mpsc`

### 1. **Неограниченная очередь**

```rust
let (tr, rs) = mpsc::channel::<i32>();   // ∞ буфер
```

**Последствия:**

- Быстрый producer → **память растёт**.
- **OOM** при большом объёме.
- **Нет backpressure**.

### 2. **Нет `select!`**

```rust
// ❌ Нельзя ждать несколько каналов одновременно
select! {
    r = rs1.recv() => { ... }
    r = rs2.recv() => { ... }
}
```

`std::sync::mpsc` **не поддерживает** `select!`. Нужно использовать:

- **отдельные** потоки;
- **`crossbeam-channel`** или **`tokio::sync::mpsc`**.

### 3. **Нет отмены**

Нет встроенного механизма **отмены** операций.

## Альтернативы

### 1. **`crossbeam-channel`**

```rust
use crossbeam_channel::{bounded, select};

let (tr, rs) = bounded::<i32>(100);   // ← bounded!

select! {
    recv(rs1) -> msg => { ... }
    recv(rs2) -> msg => { ... }
    send(tr) -> res => { ... }
    default => { ... }
}
```

**Плюсы:**

- **bounded** каналы (с backpressure).
- **`select!`** для нескольких каналов.
- **`Send + Sync`** для обоих.
- **Отмена** через `disconnect`.

### 2. **`tokio::sync::mpsc`**

```rust
use tokio::sync::mpsc;

let (tr, rs) = mpsc::channel::<i32>(100);   // ← bounded

tokio::select! {
    Some(msg) = rs1.recv() => { ... }
    Some(msg) = rs2.recv() => { ... }
}
```

**Плюсы:**

- **bounded** + **unbounded**.
- **`select!`**.
- **Асинхронный**.
- **Backpressure** через `.await`.

### 3. **`flume`**

```rust
use flume;

let (tr, rs) = flume::bounded::<i32>(100);
```

**Плюсы:**

- **Sync + async** API.
- **`select!`**.
- **Bounded** и **unbounded**.

## Сравнение каналов

| | `std::sync::mpsc` | `crossbeam-channel` | `tokio::sync::mpsc` |
|---|---|---|---|
| **Bounded** | ❌ Нет | ✅ Да | ✅ Да |
| **Unbounded** | ✅ Да (только) | ✅ Да | ✅ Да |
| **`select!`** | ❌ Нет | ✅ Да | ✅ Да |
| **Отмена** | ❌ | ✅ | ✅ |
| **Async** | ❌ | ❌ | ✅ |
| **`Receiver: Sync`** | ❌ Нет | ✅ Да | ✅ Да |
| **Много получателей** | ❌ | ✅ | ❌ (только 1) |
| **Зависимости** | std | внешняя | tokio |

## Когда использовать `std::sync::mpsc`

**Подходит:**

- **Простые** случаи.
- **Нет** внешних зависимостей.
- **Один** получатель.
- **Неограниченный** буфер **нормален**.
- **Нет** `select!`.

**Не подходит:**

- Нужен **bounded** канал (**backpressure**).
- Нужен **`select!`**.
- Нужна **отмена**.
- **Много** получателей.
- **Async**-код.

## Практический пример: bounded через `crossbeam`

```rust
use crossbeam_channel::bounded;
use std::thread;

fn main() {
    let (tr, rs) = bounded::<i32>(10);   // ← буфер 10

    let producer = thread::spawn(move || {
        for i in 0..100 {
            tr.send(i).unwrap();   // ← БЛОКИРУЕТСЯ, если буфер полон
        }
    });

    let consumer = thread::spawn(move || {
        for r in rs {
            println!("{}", r);
            thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    producer.join().unwrap();
    consumer.join().unwrap();
}
```

**Ключевое:** `send` **блокируется**, когда буфер полон → **backpressure** → **память не растёт**.

## Сводная таблица

| Аспект | `std::sync::mpsc` |
|---|---|
| **Производители** | Много (через `clone`) |
| **Потребители** | Один (`!Clone`) |
| **Очередь** | **Неограниченная** |
| **Передача** | По значению |
| **`Sender`** | `Send + Sync + Clone` |
| **`Receiver`** | `Send`, **`!Sync`**, `!Clone` |
| **`select!`** | ❌ |
| **Bounded** | ❌ |
| **Async** | ❌ |
| **Зависимости** | std |

## Итог

- **`std::sync::mpsc`** — **много отправителей → один получатель**.
- **`Sender`** — `Send + Sync + Clone`.
- **`Receiver`** — `Send`, **`!Sync`**, **`!Clone`**.
- **Очередь неограниченная** → **опасно** при медленном получателе.
- **Нет `select!`, bounded, отмены**.
- **`drop(tr)`** — **обязателен**, иначе `for r in rs` **зависнет**.
- **Для production** — `crossbeam-channel` или `tokio::sync::mpsc`:
  - **bounded** — **backpressure**;
  - **`select!`** — мультиплексирование;
  - **отмена** — graceful shutdown.
- **Правило:** `std::sync::mpsc` — для **простых** случаев; для **сложных** — **внешние** библиотеки.
