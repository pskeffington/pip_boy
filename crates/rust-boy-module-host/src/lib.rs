//! Native, transport-independent module host contract.
//! Source crates own audio/video/network workers; Rust-Boy owns permissions,
//! mounting, navigation and shutdown. No worker is started by registration.
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleId {
    Radio,
    Stream,
    Irc,
}

impl ModuleId {
    pub const ALL: [Self; 3] = [Self::Radio, Self::Stream, Self::Irc];
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Radio => "rust-radio",
            Self::Stream => "rust-stream",
            Self::Irc => "rusti-rc",
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Radio => "RADIO",
            Self::Stream => "MEDIA",
            Self::Irc => "IRC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    Unavailable,
    Ready,
    Active,
    Stopping,
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleCapability {
    ReceiveAudio,
    ReceiveVideo,
    UntrustedChat,
}

impl ModuleId {
    pub const fn capability(self) -> ModuleCapability {
        match self {
            Self::Radio => ModuleCapability::ReceiveAudio,
            Self::Stream => ModuleCapability::ReceiveVideo,
            Self::Irc => ModuleCapability::UntrustedChat,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    Denied,
    NotReady,
    Busy,
    Faulted,
}

/// A module's lifecycle is explicit and never auto-connects during construction.
/// Implementations MUST make stop idempotent, cancel worker activity and release
/// sockets/devices before reporting Ready. Drop is a final safety net.
pub trait NativeModule {
    fn id(&self) -> ModuleId;
    fn state(&self) -> ModuleState;
    fn request_stop(&mut self);
    /// Return true only once all owned resources have been released.
    fn poll_stopped(&mut self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModulePolicy {
    pub allow_audio_receive: bool,
    pub allow_video_receive: bool,
    pub allow_untrusted_chat: bool,
}

impl Default for ModulePolicy {
    fn default() -> Self {
        Self {
            allow_audio_receive: false,
            allow_video_receive: false,
            allow_untrusted_chat: false,
        }
    }
}

impl ModulePolicy {
    pub fn permits(self, id: ModuleId) -> bool {
        match id.capability() {
            ModuleCapability::ReceiveAudio => self.allow_audio_receive,
            ModuleCapability::ReceiveVideo => self.allow_video_receive,
            ModuleCapability::UntrustedChat => self.allow_untrusted_chat,
        }
    }
}

/// Safe to invoke before any backend has been attached. An unavailable module
/// cannot be described as ready or live.
pub fn available_modules<'a>(modules: impl IntoIterator<Item = &'a dyn NativeModule>) -> Vec<ModuleId> {
    modules.into_iter().filter(|m| m.state() != ModuleState::Unavailable).map(|m| m.id()).collect()
}

pub fn authorize_start(module: &dyn NativeModule, policy: ModulePolicy) -> Result<(), HostError> {
    if !policy.permits(module.id()) {
        return Err(HostError::Denied);
    }
    match module.state() {
        ModuleState::Ready => Ok(()),
        ModuleState::Unavailable => Err(HostError::NotReady),
        ModuleState::Active | ModuleState::Stopping => Err(HostError::Busy),
        ModuleState::Faulted => Err(HostError::Faulted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        id: ModuleId,
        state: ModuleState,
    }
    impl NativeModule for Fixture {
        fn id(&self) -> ModuleId { self.id }
        fn state(&self) -> ModuleState { self.state }
        fn request_stop(&mut self) { self.state = ModuleState::Stopping; }
        fn poll_stopped(&mut self) -> bool {
            self.state = ModuleState::Ready;
            true
        }
    }

    #[test]
    fn modules_default_to_deny() {
        for id in ModuleId::ALL {
            let module = Fixture { id, state: ModuleState::Ready };
            assert_eq!(authorize_start(&module, ModulePolicy::default()), Err(HostError::Denied));
        }
    }

    #[test]
    fn unconfigured_module_is_not_live() {
        let fixture = Fixture { id: ModuleId::Stream, state: ModuleState::Unavailable };
        let policy = ModulePolicy { allow_video_receive: true, ..Default::default() };
        assert_eq!(authorize_start(&fixture, policy), Err(HostError::NotReady));
        assert!(available_modules([&fixture as &dyn NativeModule]).is_empty());
    }

    #[test]
    fn active_or_stopping_sessions_cannot_be_started_twice() {
        let policy = ModulePolicy { allow_audio_receive: true, ..Default::default() };
        for state in [ModuleState::Active, ModuleState::Stopping] {
            let fixture = Fixture { id: ModuleId::Radio, state };
            assert_eq!(authorize_start(&fixture, policy), Err(HostError::Busy));
        }
    }

    #[test]
    fn independent_module_caps() {
        let policy = ModulePolicy { allow_untrusted_chat: true, ..Default::default() };
        assert!(policy.permits(ModuleId::Irc));
        assert!(!policy.permits(ModuleId::Radio));
        assert!(!policy.permits(ModuleId::Stream));
    }
}
