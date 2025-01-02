use tokio::sync::Notify;

#[async_trait::async_trait]
pub trait StorageEventHappen {
    async fn mem_change();

    async fn persistent_change();

    async fn write_pattern_change();

    async fn read_pattern_change();
}

pub struct LSMEventNotify {
    event: LSMEvent,
    notify: Notify,
}

pub enum LSMEvent {
    MemtableFull(MemtableFullEvent),
    MemtableSwitch(MemtableSwitchEvent),
    CompactionStart(CompactionStartEvent),
    CompactionPending(CompactionPendingEvent),
    CompactionEnd(CompactionEndEvent)
}

pub struct MemtableFullEvent {
    memtable_use_size: usize,
    current_instance_memory_usage: usize,
    current_write_speed: usize,
}

pub struct MemtableSwitchEvent {
    is_delay: bool,
    is_prev_freezed: bool,
    new_memtable_size: usize,
}

struct FileHandle;

enum CompactionStrategy {
    Mixed,
    WithTTL,
    Leveled,
}

enum ChosenFileStrategy {
    SmallFileHighPriority,
    LargeFileHighPriority,
}

pub struct CompactionStartEvent {
    is_delay: bool,
    chosen_files: Vec<FileHandle>,
    chosen_strategy: ChosenFileStrategy,
    compaction_strategy: CompactionStrategy,
}

pub struct CompactionPendingEvent {
    is_holding: bool,
}

pub struct CompactionEndEvent {
    is_delay: bool,
}