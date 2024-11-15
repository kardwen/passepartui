use std::{
    collections::HashMap,
    thread::{self, JoinHandle},
};

/// Runs the provided function in a new thread if an operation
/// with the same id and pass_id is not already running
pub fn run_once<'a>(
    ops_map: &mut HashMap<&'a str, (JoinHandle<()>, String)>,
    id: &'a str,
    pass_id: String,
    thread_function: impl FnOnce() + Send + 'static,
) -> bool {
    if let Some((handle, last_pass_id)) = ops_map.get(&id) {
        if &pass_id == last_pass_id && !handle.is_finished() {
            return false;
        }
    }
    let handle = thread::spawn(thread_function);
    ops_map.insert(id, (handle, pass_id));
    true
}
