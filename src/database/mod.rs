mod statistics;
mod task;

pub use statistics::{fail_statistics, finish_statistics, get_statistics};
pub use task::{fail_task, finish_task, get_task, write_signal};
