use std::{os::fd::{AsRawFd, RawFd}, time::Duration};
use bitflags::bitflags;

#[allow(unused_macros)]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

#[repr(i32)]
enum Ctl {
    /// Indicates an addition to the interest list.
    Add = libc::EPOLL_CTL_ADD,
    /// Indicates a modification of flags for an interest already in list.
    Mod = libc::EPOLL_CTL_MOD,
    /// Indicates a removal of an interest from the list.
    Del = libc::EPOLL_CTL_DEL,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct EventFlags: u32 {
        /// Sets the Edge Triggered behavior for the associated file descriptor.
        ///
        /// The default behavior for epoll is Level Triggered.
        const EPOLLET      = libc::EPOLLET as u32;
        /// The associated file is available for read operations.
        const EPOLLIN      = libc::EPOLLIN as u32;
        /// Error condition happened on the associated file descriptor.
        /// `wait` will always wait for this event; is not necessary to set it in events.
        const EPOLLERR     = libc::EPOLLERR as u32;
        /// Hang up happened on the associated file descriptor.
        ///
        /// `wait` will always wait for this event; it is not necessary to set it in events.
        /// Note that when reading from a channel such as a pipe or a stream socket, this event
        /// merely indicates that the peer closed its end of the channel. Subsequent reads from
        /// the channel will return 0 (end of file) only after all outstanding data in the
        /// channel has been consumed.
        const EPOLLHUP     = libc::EPOLLHUP as u32;
        /// The associated file is available for write operations.
        const EPOLLOUT     = libc::EPOLLOUT as u32;
        /// There is urgent data available for read operations.
        const EPOLLPRI     = libc::EPOLLPRI as u32;
        /// Stream socket peer closed connection, or shut down writing half of connection.
        ///
        /// This flag is especially useful for writing simple code to detect peer shutdown when
        /// using Edge Triggered monitoring.
        const EPOLLRDHUP   = libc::EPOLLRDHUP as u32;
        /// If `EPOLLONESHOT` and `EPOLLET` are clear and the process has the `CAP_BLOCK_SUSPEND`
        /// capability, ensure that the system does not enter "suspend" or "hibernate" while this
        /// event is pending or being processed.
        ///
        /// The event is considered as being "processed" from the time when it is returned by
        /// a call to `wait` until the next call to `wait` on the same `EpollInstance`
        /// descriptor, the closure of that file descriptor, the removal of the event file
        /// descriptor with `EPOLL_CTL_DEL`, or the clearing of `EPOLLWAKEUP` for the event file
        /// descriptor with `EPOLL_CTL_MOD`.
        const EPOLLWAKEUP  = libc::EPOLLWAKEUP as u32;
        /// Sets the one-shot behavior for the associated file descriptor.
        ///
        /// This means that after an event is pulled out with `wait` the associated file
        /// descriptor is internally disabled and no other events will be reported by the epoll
        /// interface.  The user must call `ctl` with `EPOLL_CTL_MOD` to rearm the file
        /// descriptor with a new event mask.
        const EPOLLONESHOT = libc::EPOLLONESHOT as u32;
        /// the target file descriptor, `fd`. When a wakeup event occurs and multiple epoll file
        /// descriptors are attached to the same target file using `EPOLLEXCLUSIVE`, one or more of
        /// the epoll file descriptors will receive an event with `wait`. The default in this
        /// scenario (when `EPOLLEXCLUSIVE` is not set) is for all epoll file descriptors to
        /// receive an event. `EPOLLEXCLUSIVE` is thus useful for avoiding thundering herd problems
        /// in certain scenarios.
        ///
        /// If the same file descriptor is in multiple epoll instances, some with the
        /// `EPOLLEXCLUSIVE` flag, and others without, then events will be provided to all epoll
        /// instances that did not specify `EPOLLEXCLUSIVE`, and at least one of the epoll
        /// instances that did specify `EPOLLEXCLUSIVE`.
        ///
        /// The following values may be specified in conjunction with `EPOLLEXCLUSIVE`: `EPOLLIN`,
        /// `EPOLLOUT`, `EPOLLWAKEUP`, and `EPOLLET`. `EPOLLHUP` and `EPOLLERR` can also be
        /// specified, but this is not required: as usual, these events are always reported if they
        /// occur, regardless of whether they are specified in `Events`. Attempts to specify other
        /// values in `Events` yield the error `EINVAL`.
        ///
        /// `EPOLLEXCLUSIVE` may be used only in an `EPOLL_CTL_ADD` operation; attempts to employ
        /// it with `EPOLL_CTL_MOD` yield an error. If `EPOLLEXCLUSIVE` has been set using `ctl`,
        /// then a subsequent `EPOLL_CTL_MOD` on the same `epfd`, `fd` pair yields an error. A call
        /// to `ctl` that specifies `EPOLLEXCLUSIVE` in `Events` and specifies the target file
        /// descriptor `fd` as an epoll instance will likewise fail. The error in all of these
        /// cases is `EINVAL`.
        ///
        /// The `EPOLLEXCLUSIVE` flag is an input flag for the `Event.events` field when calling
        /// `ctl`; it is never returned by `wait`.
        const EPOLLEXCLUSIVE = libc::EPOLLEXCLUSIVE as u32;
    }
}


pub struct Event {
    pub fd: RawFd,
    pub flags: EventFlags,
}

pub struct Epoll {
    epoll_fd: RawFd,
    events: Vec<libc::epoll_event>
}

impl Epoll {
    pub fn create() -> std::io::Result<Self> {
        let epoll_fd = syscall!(epoll_create(1))?;
        Ok(Self {
            epoll_fd: epoll_fd as RawFd,
            events: Vec::with_capacity(1024)
        })
    }

    #[inline]
    pub fn add(&self, fd: &impl AsRawFd, flags: EventFlags) -> std::io::Result<()> {
        let mut event = libc::epoll_event { events: flags.bits(), u64: fd.as_raw_fd() as u64 };
        self._ctl(Ctl::Add, fd.as_raw_fd(), &mut event as *mut _)
    }

    #[inline]
    pub fn modify(&self, fd: &impl AsRawFd, flags: EventFlags) -> std::io::Result<()> {
        let mut event = libc::epoll_event { events: flags.bits(), u64: fd.as_raw_fd() as u64 };
        self._ctl(Ctl::Mod, fd.as_raw_fd(), &mut event as *mut _)
    }

    #[inline]
    pub fn delete(&self, fd: &impl AsRawFd) -> std::io::Result<()> {
        self._ctl(Ctl::Del, fd.as_raw_fd(), std::ptr::null_mut())
    }

    pub fn wait(&mut self, timeout: Duration) -> std::io::Result<Vec<Event>> {
        self.events.clear();
        self.events.resize(1024, libc::epoll_event { events: 0, u64: 0 });
        let event_len = syscall!(epoll_wait(self.epoll_fd, self.events.as_mut_ptr(), self.events.len() as i32, timeout.as_millis() as i32))?;
        Ok(self.events[0..event_len as usize]
            .as_ref()
            .iter()
            .map(|e| Event{ fd: e.u64 as RawFd, flags: EventFlags::from_bits_truncate(e.events) }).collect())
    }

    #[inline]
    fn _ctl(&self, option: Ctl, fd: RawFd, event: *mut libc::epoll_event) -> std::io::Result<()> {
        syscall!(epoll_ctl(self.epoll_fd, option as i32, fd, event))?;
        Ok(())
    }
}

