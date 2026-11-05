//! Offline validation and generation contract for a future Meshtastic LAN
//! receive broker. This module performs no DNS, socket, credential-store, or
//! radio I/O and does not mint OS authorization.
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::time::{Duration, Instant};

pub const MAX_SESSION: Duration = Duration::from_secs(300);
pub const MAX_TOPIC_BYTES: usize = 128;
pub const MAX_MESSAGES_PER_SECOND: usize = 10;
pub const MESSAGE_BURST: usize = 20;
pub const MAX_PACKET_BYTES: usize = 4096;
pub const MAX_FROM_RADIO_BYTES: usize = 512;
pub const MAX_QUEUE_MESSAGES: usize = 16;
pub const MAX_QUEUE_BYTES: usize = 65_536;
pub const MAX_SESSION_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeError {
    InvalidPort,
    NonLiteralAddress,
    InvalidAddressClass,
    InvalidInterfaceScope,
    InvalidPin,
    InvalidTopic,
    InvalidDuration,
    InvalidGeneration,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReceiveProtocol {
    Mqtt311Tls,
}

/// Literal IP endpoint. IPv4-mapped IPv6 addresses are canonicalized to IPv4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiteralEndpoint(SocketAddr);

impl LiteralEndpoint {
    pub fn parse(ip: &str, port: u16, interface_scope: Option<u32>) -> Result<Self, ScopeError> {
        if port == 0 {
            return Err(ScopeError::InvalidPort);
        }
        let parsed = ip
            .parse::<IpAddr>()
            .map_err(|_| ScopeError::NonLiteralAddress)?;
        let addr = match parsed {
            IpAddr::V4(ipv4) => {
                if interface_scope.is_some() || forbidden_v4(ipv4) {
                    return Err(if interface_scope.is_some() {
                        ScopeError::InvalidInterfaceScope
                    } else {
                        ScopeError::InvalidAddressClass
                    });
                }
                SocketAddr::V4(SocketAddrV4::new(ipv4, port))
            }
            IpAddr::V6(ipv6) => {
                if let Some(mapped) = ipv6.to_ipv4_mapped() {
                    if interface_scope.is_some() || forbidden_v4(mapped) {
                        return Err(if interface_scope.is_some() {
                            ScopeError::InvalidInterfaceScope
                        } else {
                            ScopeError::InvalidAddressClass
                        });
                    }
                    SocketAddr::V4(SocketAddrV4::new(mapped, port))
                } else {
                    let scope = interface_scope.unwrap_or(0);
                    if ipv6.is_unicast_link_local() {
                        if scope == 0 {
                            return Err(ScopeError::InvalidInterfaceScope);
                        }
                    } else if scope != 0 {
                        return Err(ScopeError::InvalidInterfaceScope);
                    }
                    if forbidden_v6(ipv6) {
                        return Err(ScopeError::InvalidAddressClass);
                    }
                    SocketAddr::V6(SocketAddrV6::new(ipv6, port, 0, scope))
                }
            }
        };
        Ok(Self(addr))
    }

    pub fn socket_addr(self) -> SocketAddr {
        self.0
    }
}

fn forbidden_v4(ip: Ipv4Addr) -> bool {
    ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() || ip == Ipv4Addr::BROADCAST
}

fn forbidden_v6(ip: Ipv6Addr) -> bool {
    ip.is_unspecified() || ip.is_loopback() || ip.is_multicast()
}

/// A byte-exact MQTT topic filter with no wildcard or control characters.
#[derive(Clone, Eq, PartialEq)]
pub struct ExactTopic(String);

impl ExactTopic {
    pub fn parse(value: &str) -> Result<Self, ScopeError> {
        if value.is_empty()
            || value.len() > MAX_TOPIC_BYTES
            || value.chars().any(|ch| {
                ch == '+'
                    || ch == '#'
                    || ch == '\0'
                    || ch == '\u{fffd}'
                    || (ch as u32) < 0x20
                    || (0x7f..=0x9f).contains(&(ch as u32))
                    || (0xfdd0..=0xfdef).contains(&(ch as u32))
                    || (ch as u32 & 0xffff == 0xfffe)
                    || (ch as u32 & 0xffff == 0xffff)
            })
        {
            return Err(ScopeError::InvalidTopic);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated parameters only. This value is not a grant and exposes no
/// transport-opening API; only a future OS broker may turn it into a session.
#[derive(Clone, Eq, PartialEq)]
pub struct ScopePlan {
    endpoint: LiteralEndpoint,
    protocol: ReceiveProtocol,
    topic: ExactTopic,
    tls_pin_sha256: [u8; 32],
    duration: Duration,
}

impl ScopePlan {
    pub fn new(
        endpoint: LiteralEndpoint,
        topic: ExactTopic,
        tls_pin_sha256: [u8; 32],
        duration: Duration,
    ) -> Result<Self, ScopeError> {
        if tls_pin_sha256 == [0; 32] {
            return Err(ScopeError::InvalidPin);
        }
        if duration.is_zero() || duration > MAX_SESSION {
            return Err(ScopeError::InvalidDuration);
        }
        Ok(Self {
            endpoint,
            protocol: ReceiveProtocol::Mqtt311Tls,
            topic,
            tls_pin_sha256,
            duration,
        })
    }

    pub fn endpoint(&self) -> LiteralEndpoint {
        self.endpoint
    }

    pub fn protocol(&self) -> ReceiveProtocol {
        self.protocol
    }

    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }

    pub fn tls_pin_sha256(&self) -> &[u8; 32] {
        &self.tls_pin_sha256
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}

/// In-process freshness/revocation guard. This is not an authenticated grant;
/// production socket opening remains unavailable.
pub struct GenerationGuard {
    plan: ScopePlan,
    generation: u64,
    deadline: Instant,
    active: bool,
}

impl GenerationGuard {
    pub fn bind_for_broker_contract(
        plan: &ScopePlan,
        generation: u64,
        now: Instant,
    ) -> Result<Self, ScopeError> {
        if generation == 0 {
            return Err(ScopeError::InvalidGeneration);
        }
        Ok(Self {
            plan: plan.clone(),
            generation,
            deadline: now + plan.duration,
            active: true,
        })
    }

    pub fn is_current(&self, plan: &ScopePlan, generation: u64, now: Instant) -> bool {
        self.active && now < self.deadline && self.generation == generation && self.plan == *plan
    }

    pub fn revoke(&mut self) {
        self.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIN: [u8; 32] = [7; 32];

    fn plan() -> ScopePlan {
        let endpoint = LiteralEndpoint::parse("172.20.30.135", 9443, None).unwrap();
        let topic = ExactTopic::parse("msh/US/2/e/LongFast/!abcdef").unwrap();
        ScopePlan::new(endpoint, topic, PIN, Duration::from_secs(60)).unwrap()
    }

    #[derive(Default)]
    struct FakeTransport {
        opens: usize,
        reads: usize,
    }

    impl FakeTransport {
        fn attempt_open(
            &mut self,
            lease: Option<&GenerationGuard>,
            plan: &ScopePlan,
            generation: u64,
            now: Instant,
        ) {
            if lease.is_some_and(|guard| guard.is_current(plan, generation, now)) {
                self.opens += 1;
                self.reads += 1;
            }
        }
    }

    #[test]
    fn invalid_or_missing_session_state_performs_zero_transport_calls() {
        let plan = plan();
        let now = Instant::now();
        let mut lease = GenerationGuard::bind_for_broker_contract(&plan, 4, now).unwrap();
        let wrong_endpoint = ScopePlan::new(
            LiteralEndpoint::parse("172.20.30.136", 9443, None).unwrap(),
            ExactTopic::parse("msh/US/2/e/LongFast/!abcdef").unwrap(),
            PIN,
            Duration::from_secs(60),
        )
        .unwrap();
        let wrong_topic = ScopePlan::new(
            plan.endpoint(),
            ExactTopic::parse("msh/US/2/e/LongFast/!other").unwrap(),
            PIN,
            Duration::from_secs(60),
        )
        .unwrap();
        let wrong_pin = ScopePlan::new(
            plan.endpoint(),
            ExactTopic::parse("msh/US/2/e/LongFast/!abcdef").unwrap(),
            [8; 32],
            Duration::from_secs(60),
        )
        .unwrap();
        let mut transport = FakeTransport::default();
        transport.attempt_open(None, &plan, 4, now);
        transport.attempt_open(Some(&lease), &wrong_endpoint, 4, now);
        transport.attempt_open(Some(&lease), &wrong_topic, 4, now);
        transport.attempt_open(Some(&lease), &wrong_pin, 4, now);
        transport.attempt_open(Some(&lease), &plan, 5, now);
        transport.attempt_open(Some(&lease), &plan, 4, now + Duration::from_secs(61));
        lease.revoke();
        transport.attempt_open(Some(&lease), &plan, 4, now);
        assert_eq!((transport.opens, transport.reads), (0, 0));
    }

    #[test]
    fn matching_unexpired_lease_only_reaches_the_fake_transport() {
        let plan = plan();
        let now = Instant::now();
        let lease = GenerationGuard::bind_for_broker_contract(&plan, 9, now).unwrap();
        let mut fake = FakeTransport::default();
        fake.attempt_open(Some(&lease), &plan, 9, now + Duration::from_secs(1));
        assert_eq!((fake.opens, fake.reads), (1, 1));
    }

    #[test]
    fn generation_zero_is_reserved_as_invalid() {
        let plan = plan();
        assert_eq!(
            GenerationGuard::bind_for_broker_contract(&plan, 0, Instant::now()).err(),
            Some(ScopeError::InvalidGeneration)
        );
    }

    #[test]
    fn endpoint_requires_literal_address_and_rejects_forbidden_classes() {
        for invalid in [
            "localhost",
            "example.invalid",
            "127.0.0.1",
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "::1",
            "::",
            "ff02::1",
            "::ffff:127.0.0.1",
        ] {
            assert!(
                LiteralEndpoint::parse(invalid, 9443, None).is_err(),
                "{invalid}"
            );
        }
        assert!(LiteralEndpoint::parse("172.20.30.135", 0, None).is_err());
        assert!(LiteralEndpoint::parse("fe80::1", 9443, None).is_err());
        assert!(LiteralEndpoint::parse("fe80::1", 9443, Some(0)).is_err());
        assert!(LiteralEndpoint::parse("fe80::1", 9443, Some(4)).is_ok());
        assert!(LiteralEndpoint::parse("2001:db8::1", 9443, Some(4)).is_err());
        assert_eq!(
            LiteralEndpoint::parse("::ffff:172.20.30.135", 9443, None)
                .unwrap()
                .socket_addr(),
            "172.20.30.135:9443".parse().unwrap()
        );
    }

    #[test]
    fn exact_topic_is_validated_and_fixed_caps_are_declared() {
        for topic in ["", "x+", "x/#", "x\0y", "x\n", "x\u{0085}", "x\u{fffd}"] {
            assert!(ExactTopic::parse(topic).is_err(), "{topic:?}");
        }
        assert!(ExactTopic::parse(&"x".repeat(MAX_TOPIC_BYTES + 1)).is_err());
        assert!(ExactTopic::parse(&"x".repeat(MAX_TOPIC_BYTES)).is_ok());
        assert_eq!(MAX_PACKET_BYTES, 4096);
        assert_eq!(MAX_FROM_RADIO_BYTES, 512);
        assert_eq!(MAX_QUEUE_MESSAGES, 16);
        assert_eq!(MAX_QUEUE_BYTES, 65_536);
        assert_eq!(MAX_SESSION_BYTES, 1_048_576);
        assert_eq!(MAX_MESSAGES_PER_SECOND, 10);
        assert_eq!(MESSAGE_BURST, 20);
    }

    #[test]
    fn plan_bounds_duration_and_rejects_empty_pin() {
        let endpoint = LiteralEndpoint::parse("172.20.30.135", 9443, None).unwrap();
        let topic = ExactTopic::parse("msh/US/2/e/LongFast/!abcdef").unwrap();
        assert!(ScopePlan::new(endpoint, topic.clone(), PIN, Duration::ZERO).is_err());
        assert!(ScopePlan::new(
            endpoint,
            topic.clone(),
            PIN,
            MAX_SESSION + Duration::from_secs(1)
        )
        .is_err());
        assert!(ScopePlan::new(endpoint, topic.clone(), [0; 32], Duration::from_secs(1)).is_err());
        assert!(ScopePlan::new(endpoint, topic, PIN, MAX_SESSION).is_ok());
    }
}
