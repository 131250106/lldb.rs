// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::broadcaster::SBBroadcaster;
use super::error::SBError;
use super::event::SBEvent;
use super::processinfo::SBProcessInfo;
use super::queue::SBQueue;
use super::stream::SBStream;
use super::thread::SBThread;
use super::{lldb_pid_t, lldb_tid_t, StateType};
use std::ffi::{CStr, CString};
use std::fmt;
use sys;

/// The process associated with the target program.
///
/// You get a process by attaching to or launching a target program.
/// See [`SBTarget`] for details.
///
/// # Process State
///
/// The OS process ID (`pid_t`) for the process is available via
/// [`process_id`].
///
/// The [process state] can be obtained via [`state`]. It is common to
/// just check to see if the process [`is_alive`], [`is_running`] or
/// [`is_stopped`].
///
/// Once the process is in the `Exited` state, the [`exit_status`] and
/// [`exit_description`] are available for inspection.
///
/// # Execution Control
///
/// Once you have a process, you can:
///
/// * [`continue_execution`]
/// * [`stop`]
/// * [`kill`]
/// * [`detach`]
///
/// # Threads
///
/// The process contains the [threads of execution] for the [target]. The
/// available threads can be iterated over with [`threads`]:
///
/// ```no_run
/// # use lldb::{SBProcess, SBThread};
/// # fn look_at_threads(process: SBProcess) {
/// // Iterate over the threads...
/// for thread in process.threads() {
///     println!("Hello {}!", thread.thread_id());
/// }
/// // Or collect them into a vector!
/// let threads = process.threads().collect::<Vec<SBThread>>();
/// # }
/// ```
///
/// Specific individual threads can be looked up via [`thread_by_id`]
/// and [`thread_by_index_id`] methods.
///
/// Some functions operate on the 'currently selected thread'. This can
/// retrieved via [`selected_thread`] and set via [`set_selected_thread`],
/// [`set_selected_thread_by_id`], or [`set_selected_thread_by_index_id`].
///
/// # Queues
///
/// A process may also have a set of queues associated with it. This is used
/// on macOS, iOS and other Apple operating systems to support debugger
/// integration with `libdispatch`, also known as GCD or "Grand Central
/// Dispatch".
///
/// The active queues can be iterated over with [`queues`]:
///
/// ```no_run
/// # use lldb::{SBProcess, SBQueue};
/// # fn look_at_queues(process: SBProcess) {
/// // Iterate over the queues...
/// for queue in process.queues() {
///     println!("Hello {}!", queue.queue_id());
/// }
/// # }
/// ```
///
/// # Events
///
/// ... to be written ...
///
/// [`SBTarget`]: struct.SBTarget.html
/// [`process_id`]: #method.process_id
/// [process state]: enum.StateType.html
/// [`state`]: #method.state
/// [`is_alive`]: #method.is_alive
/// [`is_running`]: #method.is_running
/// [`is_stopped`]: #method.is_stopped
/// [`exit_status`]: #method.exit_status
/// [`exit_description`]: #method.exit_description
/// [`continue_execution`]: #method.continue_execution
/// [`stop`]: #method.stop
/// [`kill`]: #method.kill
/// [`detach`]: #method.detach
/// [threads of execution]: struct.SBThread.html
/// [target]: struct.SBTarget.html
/// [`threads`]: #method.threads
/// [`thread_by_id`]: #method.thread_by_id
/// [`thread_by_index_id`]: #method.thread_by_index_id
/// [`selected_thread`]: #method.selected_thread
/// [`set_selected_thread`]: #method.set_selected_thread
/// [`set_selected_thread_by_id`]: #method.set_selected_thread_by_id
/// [`set_selected_thread_by_index_id`]: #method.set_selected_thread_by_index_id
/// [`queues`]: #method.queues
pub struct SBProcess {
    /// The underlying raw `SBProcessRef`.
    pub raw: sys::SBProcessRef,
}

impl SBProcess {
    /// Construct a new `SBProcess`.
    pub fn wrap(raw: sys::SBProcessRef) -> SBProcess {
        SBProcess { raw }
    }

    /// Construct a new `Some(SBProcess)` or `None`.
    pub fn maybe_wrap(raw: sys::SBProcessRef) -> Option<SBProcess> {
        if unsafe { sys::SBProcessIsValid(raw) != 0 } {
            Some(SBProcess { raw })
        } else {
            None
        }
    }

    /// Check whether or not this is a valid `SBProcess` value.
    pub fn is_valid(&self) -> bool {
        unsafe { sys::SBProcessIsValid(self.raw) != 0 }
    }

    #[allow(missing_docs)]
    pub fn broadcaster_class_name() -> &'static str {
        unsafe {
            match CStr::from_ptr(sys::SBProcessGetBroadcasterClassName()).to_str() {
                Ok(s) => s,
                _ => panic!("Invalid string?"),
            }
        }
    }

    /// The current state of this process (running, stopped, exited, etc.).
    pub fn state(&self) -> StateType {
        unsafe { sys::SBProcessGetState(self.raw) }
    }

    /// Returns `true` if the process is currently alive.
    ///
    /// This corresponds to the process being in the `Attaching`,
    /// `Launching`, `Stopped`, `Running`, `Stepping`, `Crashed`
    /// or `Suspended` states.
    pub fn is_alive(&self) -> bool {
        match self.state() {
            StateType::Attaching
            | StateType::Launching
            | StateType::Stopped
            | StateType::Running
            | StateType::Stepping
            | StateType::Crashed
            | StateType::Suspended => true,
            _ => false,
        }
    }

    /// Returns `true` if the process is currently running.
    ///
    /// This corresponds to the process being in the `Running`
    /// or `Stepping` states.
    pub fn is_running(&self) -> bool {
        match self.state() {
            StateType::Running | StateType::Stepping => true,
            _ => false,
        }
    }

    /// Returns `true` if the process is currently stopped.
    ///
    /// This corresponds to the process being in the `Stopped`, `Crashed`,
    /// or `Suspended` states.
    pub fn is_stopped(&self) -> bool {
        match self.state() {
            StateType::Stopped | StateType::Crashed | StateType::Suspended => true,
            _ => false,
        }
    }

    /// The exit status of the process when the process state is
    /// `Exited`.
    pub fn exit_status(&self) -> i32 {
        unsafe { sys::SBProcessGetExitStatus(self.raw) }
    }

    /// The exit description of the process when the process state
    /// is `Exited`.
    pub fn exit_description(&self) -> &str {
        unsafe {
            match CStr::from_ptr(sys::SBProcessGetExitDescription(self.raw)).to_str() {
                Ok(s) => s,
                _ => panic!("Invalid string?"),
            }
        }
    }

    /// Returns the process ID of the process.
    pub fn process_id(&self) -> lldb_pid_t {
        unsafe { sys::SBProcessGetProcessID(self.raw) }
    }

    /// Returns an integer ID that is guaranteed to be unique across all
    /// process instances. This is not the process ID, just a unique
    /// integer for comparison and caching purposes.
    pub fn unique_id(&self) -> u32 {
        unsafe { sys::SBProcessGetUniqueID(self.raw) }
    }

    /// Get the size, in bytes, of an address.
    pub fn address_byte_size(&self) -> u32 {
        unsafe { sys::SBProcessGetAddressByteSize(self.raw) }
    }

    /// Kills the process and shuts down all threads that were spawned to
    /// track and monitor the process.
    pub fn destroy(&self) -> Result<(), SBError> {
        let error = SBError::wrap(unsafe { sys::SBProcessDestroy(self.raw) });
        if error.is_success() {
            Ok(())
        } else {
            Err(error)
        }
    }

    #[allow(missing_docs)]
    pub fn continue_execution(&self) -> Result<(), SBError> {
        let error = SBError::wrap(unsafe { sys::SBProcessContinue(self.raw) });
        if error.is_success() {
            Ok(())
        } else {
            Err(error)
        }
    }

    #[allow(missing_docs)]
    pub fn stop(&self) -> Result<(), SBError> {
        let error = SBError::wrap(unsafe { sys::SBProcessStop(self.raw) });
        if error.is_success() {
            Ok(())
        } else {
            Err(error)
        }
    }

    /// Same as calling `destroy`.
    pub fn kill(&self) -> Result<(), SBError> {
        let error = SBError::wrap(unsafe { sys::SBProcessKill(self.raw) });
        if error.is_success() {
            Ok(())
        } else {
            Err(error)
        }
    }

    #[allow(missing_docs)]
    pub fn detach(&self) -> Result<(), SBError> {
        let error = SBError::wrap(unsafe { sys::SBProcessDetach(self.raw) });
        if error.is_success() {
            Ok(())
        } else {
            Err(error)
        }
    }

    /// Send the process a Unix signal.
    pub fn signal(&self, signal: i32) -> Result<(), SBError> {
        let error = SBError::wrap(unsafe { sys::SBProcessSignal(self.raw, signal) });
        if error.is_success() {
            Ok(())
        } else {
            Err(error)
        }
    }

    #[allow(missing_docs)]
    pub fn broadcaster(&self) -> SBBroadcaster {
        SBBroadcaster::wrap(unsafe { sys::SBProcessGetBroadcaster(self.raw) })
    }

    /// Get an iterator over the [threads] known to this process instance.
    ///
    /// [threads]: struct.SBThread.html
    pub fn threads(&self) -> SBProcessThreadIter {
        SBProcessThreadIter {
            process: self,
            idx: 0,
        }
    }

    /// Get an iterator over the [queues] known to this process instance.
    ///
    /// [queues]: struct.SBQueue.html
    pub fn queues(&self) -> SBProcessQueueIter {
        SBProcessQueueIter {
            process: self,
            idx: 0,
        }
    }

    /// Returns the thread with the given thread ID.
    pub fn thread_by_id(&self, thread_id: lldb_tid_t) -> Option<SBThread> {
        SBThread::maybe_wrap(unsafe { sys::SBProcessGetThreadByID(self.raw, thread_id) })
    }

    /// Returns the thread with the given thread index ID.
    pub fn thread_by_index_id(&self, thread_index_id: u32) -> Option<SBThread> {
        SBThread::maybe_wrap(unsafe { sys::SBProcessGetThreadByIndexID(self.raw, thread_index_id) })
    }

    /// Returns the currently selected thread.
    pub fn selected_thread(&self) -> SBThread {
        SBThread::wrap(unsafe { sys::SBProcessGetSelectedThread(self.raw) })
    }

    /// Set the selected thread.
    pub fn set_selected_thread(&self, thread: &SBThread) -> bool {
        unsafe { sys::SBProcessSetSelectedThread(self.raw, thread.raw) != 0 }
    }

    /// Set the selected thread by ID.
    pub fn set_selected_thread_by_id(&self, thread_id: lldb_tid_t) -> bool {
        unsafe { sys::SBProcessSetSelectedThreadByID(self.raw, thread_id) != 0 }
    }

    /// Set the selected thread by index ID.
    pub fn set_selected_thread_by_index_id(&self, thread_index_id: u32) -> bool {
        unsafe { sys::SBProcessSetSelectedThreadByIndexID(self.raw, thread_index_id) != 0 }
    }

    #[allow(missing_docs)]
    pub fn event_as_process_event(event: &SBEvent) -> Option<SBProcessEvent> {
        if unsafe { sys::SBProcessEventIsProcessEvent(event.raw) != 0 } {
            Some(SBProcessEvent::new(event))
        } else {
            None
        }
    }

    /// Save the state of the process in a core file (or mini dump on Windows).
    pub fn save_core(&self, file_name: &str) -> Result<(), SBError> {
        let f = CString::new(file_name).unwrap();
        let error = SBError::wrap(unsafe { sys::SBProcessSaveCore(self.raw, f.as_ptr()) });
        if error.is_success() {
            Ok(())
        } else {
            Err(error)
        }
    }

    #[allow(missing_docs)]
    pub fn process_info(&self) -> SBProcessInfo {
        SBProcessInfo::wrap(unsafe { sys::SBProcessGetProcessInfo(self.raw) })
    }
}

/// Iterate over the [threads] in a [process].
///
/// [threads]: struct.SBThread.html
/// [process]: struct.SBProcess.html
pub struct SBProcessThreadIter<'d> {
    process: &'d SBProcess,
    idx: usize,
}

impl<'d> Iterator for SBProcessThreadIter<'d> {
    type Item = SBThread;

    fn next(&mut self) -> Option<SBThread> {
        if self.idx < unsafe { sys::SBProcessGetNumThreads(self.process.raw) as usize } {
            let r = Some(SBThread::wrap(unsafe {
                sys::SBProcessGetThreadAtIndex(self.process.raw, self.idx)
            }));
            self.idx += 1;
            r
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let sz = unsafe { sys::SBProcessGetNumThreads(self.process.raw) } as usize;
        (sz - self.idx, Some(sz))
    }
}

/// Iterate over the [queues] in a [process].
///
/// [queues]: struct.SBQueue.html
/// [process]: struct.SBProcess.html
pub struct SBProcessQueueIter<'d> {
    process: &'d SBProcess,
    idx: usize,
}

impl<'d> Iterator for SBProcessQueueIter<'d> {
    type Item = SBQueue;

    fn next(&mut self) -> Option<SBQueue> {
        if self.idx < unsafe { sys::SBProcessGetNumQueues(self.process.raw) as usize } {
            let r = Some(SBQueue::wrap(unsafe {
                sys::SBProcessGetQueueAtIndex(self.process.raw, self.idx)
            }));
            self.idx += 1;
            r
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let sz = unsafe { sys::SBProcessGetNumQueues(self.process.raw) } as usize;
        (sz - self.idx, Some(sz))
    }
}

impl fmt::Debug for SBProcess {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let stream = SBStream::new();
        unsafe { sys::SBProcessGetDescription(self.raw, stream.raw) };
        write!(fmt, "SBProcess {{ {} }}", stream.data())
    }
}

impl Drop for SBProcess {
    fn drop(&mut self) {
        unsafe { sys::DisposeSBProcess(self.raw) };
    }
}

#[allow(missing_docs)]
pub struct SBProcessEvent<'e> {
    event: &'e SBEvent,
}

#[allow(missing_docs)]
impl<'e> SBProcessEvent<'e> {
    pub fn new(event: &'e SBEvent) -> Self {
        SBProcessEvent { event }
    }

    pub fn process_state(&self) -> StateType {
        unsafe { sys::SBProcessGetStateFromEvent(self.event.raw) }
    }

    pub fn process(&self) -> SBProcess {
        SBProcess::wrap(unsafe { sys::SBProcessGetProcessFromEvent(self.event.raw) })
    }

    pub fn interrupted(&self) -> bool {
        unsafe { sys::SBProcessGetInterruptedFromEvent(self.event.raw) != 0 }
    }

    pub fn restarted(&self) -> bool {
        unsafe { sys::SBProcessGetRestartedFromEvent(self.event.raw) != 0 }
    }

    pub fn restarted_reasons(&self) -> SBProcessEventRestartedReasonIter {
        SBProcessEventRestartedReasonIter {
            event: self,
            idx: 0,
        }
    }
}

/// Iterate over the restart reasons in a [process event].
///
/// [process event]: struct.SBProcessEvent.html
pub struct SBProcessEventRestartedReasonIter<'d> {
    event: &'d SBProcessEvent<'d>,
    idx: usize,
}

impl<'d> Iterator for SBProcessEventRestartedReasonIter<'d> {
    type Item = &'d str;

    fn next(&mut self) -> Option<&'d str> {
        let raw = self.event.event.raw;
        if self.idx < unsafe { sys::SBProcessGetNumRestartedReasonsFromEvent(raw) as usize } {
            let r = unsafe {
                let s = CStr::from_ptr(sys::SBProcessGetRestartedReasonAtIndexFromEvent(
                    raw, self.idx,
                ));
                match s.to_str() {
                    Ok(s) => s,
                    _ => panic!("Invalid string?"),
                }
            };
            self.idx += 1;
            Some(r)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let sz =
            unsafe { sys::SBProcessGetNumRestartedReasonsFromEvent(self.event.event.raw) } as usize;
        (sz - self.idx, Some(sz))
    }
}

impl<'d> ExactSizeIterator for SBProcessEventRestartedReasonIter<'d> {}

#[cfg(feature = "graphql")]
graphql_object!(SBProcess: super::debugger::SBDebugger | &self | {
    field is_valid() -> bool {
        self.is_valid()
    }

    field is_alive() -> bool {
        self.is_alive()
    }

    field is_running() -> bool {
        self.is_running()
    }

    field is_stopped() -> bool {
        self.is_stopped()
    }

    // TODO(bm) This should be i32
    field exit_status() -> i64 {
        self.exit_status() as i64
    }

    field exit_description() -> &str {
        self.exit_description()
    }

    // TODO(bm): This should be u64
    field process_id() -> i64 {
        self.process_id() as i64
    }

    // TODO(bm) This should be u32
    field unique_id() -> i64 {
        self.unique_id() as i64
    }

    // TODO(bm) This should be u32
    field address_byte_size() -> i64 {
        self.address_byte_size() as i64
    }

    field threads() -> Vec<SBThread> {
        self.threads().collect()
    }

    field queues() -> Vec<SBQueue> {
        self.queues().collect()
    }

    field selected_thread() -> SBThread {
        self.selected_thread()
    }

    field process_info() -> SBProcessInfo {
        self.process_info()
    }
});
