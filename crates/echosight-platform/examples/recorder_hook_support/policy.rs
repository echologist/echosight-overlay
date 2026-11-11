//! Allocation-free policy shared by the native adapter and executable unit tests.
//! No Windows/GPUI dependencies, OS calls, clocks, pointers or business-rule clone.
pub const CAPACITY: usize = 64;
pub const TAIL_MS: u64 = 50;
pub const TAIL_CHARS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Physical(pub usize);
impl Physical {
    pub fn from_raw(vk: u32, bits: u32) -> Self {
        let scan = (bits >> 16) & 255;
        Self(if scan == 0 {
            512 + (vk & 255) as usize
        } else {
            (scan | ((bits >> 16) & 256)) as usize
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Down,
    Up,
    Character,
    Other,
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub hwnd: usize,
    pub kind: Kind,
    pub key: Physical,
    pub repeat: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    pub consume: bool,
    pub capture: bool,
    pub arm: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotStatus {
    Ok,
    Failed,
    MissingLayout,
}
#[derive(Clone, Copy, Debug)]
pub struct Raw {
    pub message: u32,
    pub vk: u32,
    pub bits: u32,
    pub time: u32,
    pub keyboard: [u8; 256],
    pub hkl: usize,
    pub status: SnapshotStatus,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Standard,
    Left,
    Right,
    Numpad,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawShape {
    pub scan: u8,
    pub extended: bool,
    pub repeat_count: u16,
    pub previously_down: bool,
    pub transition_up: bool,
    pub alt_context: bool,
    pub location: Location,
}
impl Raw {
    /// Native physical location hint, NOT DOM KeyboardEvent.location parity.
    /// Original VK/bits remain authoritative, including scan-zero/VK_PACKET.
    pub fn shape(&self) -> RawShape {
        let scan = ((self.bits >> 16) & 255) as u8;
        let extended = self.bits & (1 << 24) != 0;
        let location = match self.vk {
            0xa0 | 0xa2 | 0xa4 | 0x5b => Location::Left,
            0xa1 | 0xa3 | 0xa5 | 0x5c => Location::Right,
            0x10 => match scan {
                0x2a => Location::Left,
                0x36 => Location::Right,
                _ => Location::Unknown,
            },
            0x11 | 0x12 => {
                if extended {
                    Location::Right
                } else {
                    Location::Left
                }
            }
            _ if scan == 0 => Location::Unknown,
            0x60..=0x6f => Location::Numpad,
            0x0d if extended => Location::Numpad,
            0x0c | 0x21..=0x28 | 0x2d | 0x2e if !extended => Location::Numpad,
            _ => Location::Standard,
        };
        RawShape {
            scan,
            extended,
            repeat_count: self.bits as u16,
            previously_down: self.bits & (1 << 30) != 0,
            transition_up: self.bits & (1 << 31) != 0,
            alt_context: self.bits & (1 << 29) != 0,
            location,
        }
    }
}
impl Default for Raw {
    fn default() -> Self {
        Self {
            message: 0,
            vk: 0,
            bits: 0,
            time: 0,
            keyboard: [0; 256],
            hkl: 0,
            status: SnapshotStatus::Ok,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub sequence: u64,
    pub arm: u64,
    pub raw: Raw,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loss {
    pub first_sequence: u64,
    pub arm: u64,
    pub count: u64,
}
pub struct Batch {
    pub records: [Option<Record>; CAPACITY],
    pub loss: Option<Loss>,
}
#[derive(Clone, Copy, Debug)]
pub enum Command {
    Begin,
    Switch,
    Save,
    Cancel,
    Stop,
    DetachedStop,
    Global,
    Close,
    FocusLost,
}

pub struct Policy {
    pub target: usize,
    pub armed: bool,
    pub arm: u64,
    protected: [bool; 768],
    tail_until: u64,
    tail_left: usize,
    queue: [Option<Record>; CAPACITY],
    len: usize,
    sequence: u64,
    loss: Option<Loss>,
}
impl Policy {
    pub const fn new(target: usize) -> Self {
        Self {
            target,
            armed: false,
            arm: 0,
            protected: [false; 768],
            tail_until: 0,
            tail_left: 0,
            queue: [None; CAPACITY],
            len: 0,
            sequence: 0,
            loss: None,
        }
    }
    pub fn decide(&mut self, code: i32, remove: bool, input: Input, now: u64) -> Decision {
        if code != 0 || !remove || self.target == 0 || input.hwnd != self.target {
            return Decision::default();
        }
        let protected = self.protected.get(input.key.0).copied().unwrap_or(false);
        let consume = match input.kind {
            Kind::Down if self.armed => {
                if let Some(slot) = self.protected.get_mut(input.key.0) {
                    *slot = true;
                }
                true
            }
            Kind::Down if !input.repeat => {
                // A fresh physical cycle supersedes stale release debt for this key.
                if let Some(slot) = self.protected.get_mut(input.key.0) {
                    *slot = false;
                }
                self.tail_left = 0;
                false
            }
            Kind::Down => protected,
            Kind::Up => {
                if let Some(slot) = self.protected.get_mut(input.key.0) {
                    *slot = false;
                }
                protected
            }
            Kind::Character => {
                if self.armed {
                    true
                } else if now < self.tail_until && self.tail_left > 0 {
                    self.tail_left -= 1;
                    true
                } else {
                    false
                }
            }
            Kind::Other => false,
        };
        if !consume {
            return Decision::default();
        }
        Decision {
            consume,
            capture: self.armed && matches!(input.kind, Kind::Down | Kind::Up),
            arm: self.arm,
        }
    }
    pub fn push(&mut self, decision: Decision, raw: Raw) {
        if !decision.capture {
            return;
        }
        self.sequence = self.sequence.saturating_add(1);
        if self.len < CAPACITY {
            self.queue[self.len] = Some(Record {
                sequence: self.sequence,
                arm: decision.arm,
                raw,
            });
            self.len += 1;
        } else {
            let loss = self.loss.get_or_insert(Loss {
                first_sequence: self.sequence,
                arm: decision.arm,
                count: 0,
            });
            loss.count = loss.count.saturating_add(1);
        }
    }
    pub fn drain(&mut self) -> Batch {
        let records = std::mem::replace(&mut self.queue, [None; CAPACITY]);
        self.len = 0;
        Batch {
            records,
            loss: self.loss.take(),
        }
    }
    /// No callbacks/OS calls: capture batch belongs to the old arm. Caller applies
    /// it before business effects for this boundary, without pumping in between.
    pub fn boundary(&mut self, command: Command, now: u64, preheld: &[Physical]) -> Batch {
        let batch = self.drain();
        match command {
            Command::Begin | Command::Switch if self.target != 0 => {
                self.arm = self.arm.saturating_add(1);
                self.armed = true;
                self.tail_left = 0;
                // Carry consumed-down release debt through a switch. Fresh Begin
                // preheld snapshot supplements it; focus loss clears it explicitly.
                for key in preheld {
                    if let Some(slot) = self.protected.get_mut(key.0) {
                        *slot = true;
                    }
                }
            }
            Command::Save | Command::Cancel | Command::Stop | Command::DetachedStop => {
                if self.armed {
                    self.armed = false;
                    self.tail_until = now.saturating_add(TAIL_MS);
                    self.tail_left = TAIL_CHARS;
                }
            }
            Command::Close => self.destroy(),
            Command::FocusLost => self.focus_lost(),
            _ => {}
        }
        batch
    }
    pub fn focus_lost(&mut self) {
        self.armed = false;
        self.protected.fill(false);
        self.tail_left = 0;
    }
    pub fn destroy(&mut self) {
        self.focus_lost();
        self.target = 0;
    }
}

/// Chain helper owns no State or MSG reference. Adapter supplies raw-pointer
/// mutation closures; a chained hook may retrieve recursively or mutate MSG.
pub fn chain_reassert<T>(
    selected: bool,
    mut neutralize: impl FnMut(),
    chain: impl FnOnce() -> T,
) -> T {
    if selected {
        neutralize();
    }
    let result = chain();
    if selected {
        neutralize();
    }
    result
}

/// Models ownership separately from whether interception is active. Never owns a pointer.
#[derive(Default, Debug)]
pub struct Lease {
    pub handle: Option<usize>,
    pub depth: usize,
    pub debt: bool,
    released: bool,
}
impl Lease {
    pub const fn new() -> Self {
        Self {
            handle: None,
            depth: 0,
            debt: false,
            released: false,
        }
    }
    pub fn installed(&mut self, handle: usize) {
        self.handle = Some(handle);
        self.released = false;
        self.debt = false;
    }
    pub fn enter(&mut self) {
        self.depth = self.depth.saturating_add(1);
    }
    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        self.reclaim();
    }
    pub fn needs_unhook(&self) -> bool {
        self.handle.is_some() && !self.released
    }
    pub fn unhook_result(&mut self, success: bool) {
        self.debt = !success && self.handle.is_some();
        if success {
            self.released = true;
        }
        self.reclaim();
    }
    fn reclaim(&mut self) {
        if self.released && self.depth == 0 {
            self.handle = None;
            self.debt = false;
        }
    }
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
