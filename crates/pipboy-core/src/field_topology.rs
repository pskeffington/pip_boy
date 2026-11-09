//! Pure local planning for a future hybrid peer/relay topology.
//! A route is only a candidate: proximity, a relay, or a reported link never
//! authenticates an endpoint or grants access to a dataset.
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bearer {
    DirectLan,
    LocalHotspot,
    ManagedRelay,
    LowBandwidthRadio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadClass {
    PublicDemo,
    Internal,
    Restricted,
    Regulated,
}

/// Untrusted, caller-reported link facts used only by this pure simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteCandidate {
    pub bearer: Bearer,
    pub reachable: bool,
    pub authenticated_peer: bool,
    pub dataset_authorized: bool,
    pub fresh_revocation: bool,
}

/// A planning result. `SimulationOnlyCandidate` must never authorize or dispatch transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteDecision {
    SimulationOnlyCandidate(Bearer),
    HoldOffline,
}

/// This is a fail-closed simulation gate, NOT an authorization decision.
/// The first deployment phase only supports synthetic/public-demo data.
pub fn choose_route(class: PayloadClass, links: &[RouteCandidate]) -> RouteDecision {
    if class != PayloadClass::PublicDemo {
        return RouteDecision::HoldOffline;
    }
    // Prefer direct authenticated links, then a managed relay. A low-bandwidth
    // radio is never selected for a general message/task payload here.
    for bearer in [
        Bearer::DirectLan,
        Bearer::LocalHotspot,
        Bearer::ManagedRelay,
    ] {
        if links.iter().any(|route| {
            route.bearer == bearer
                && route.reachable
                && route.authenticated_peer
                && route.dataset_authorized
                && route.fresh_revocation
        }) {
            return RouteDecision::SimulationOnlyCandidate(bearer);
        }
    }
    RouteDecision::HoldOffline
}

#[cfg(test)]
mod tests {
    use super::*;
    const TRUSTED: RouteCandidate = RouteCandidate {
        bearer: Bearer::DirectLan,
        reachable: true,
        authenticated_peer: true,
        dataset_authorized: true,
        fresh_revocation: true,
    };

    #[test]
    fn no_routes_means_offline_hold() {
        assert_eq!(
            choose_route(PayloadClass::PublicDemo, &[]),
            RouteDecision::HoldOffline
        );
    }

    #[test]
    fn each_security_gate_must_pass() {
        for broken in [
            RouteCandidate {
                reachable: false,
                ..TRUSTED
            },
            RouteCandidate {
                authenticated_peer: false,
                ..TRUSTED
            },
            RouteCandidate {
                dataset_authorized: false,
                ..TRUSTED
            },
            RouteCandidate {
                fresh_revocation: false,
                ..TRUSTED
            },
        ] {
            assert_eq!(
                choose_route(PayloadClass::PublicDemo, &[broken]),
                RouteDecision::HoldOffline
            );
        }
    }

    #[test]
    fn direct_link_wins_over_relay() {
        let relay = RouteCandidate {
            bearer: Bearer::ManagedRelay,
            ..TRUSTED
        };
        assert_eq!(
            choose_route(PayloadClass::PublicDemo, &[relay, TRUSTED]),
            RouteDecision::SimulationOnlyCandidate(Bearer::DirectLan)
        );
    }

    #[test]
    fn hotspot_precedes_managed_relay() {
        let relay = RouteCandidate {
            bearer: Bearer::ManagedRelay,
            ..TRUSTED
        };
        let hotspot = RouteCandidate {
            bearer: Bearer::LocalHotspot,
            ..TRUSTED
        };
        assert_eq!(
            choose_route(PayloadClass::PublicDemo, &[relay, hotspot]),
            RouteDecision::SimulationOnlyCandidate(Bearer::LocalHotspot)
        );
    }

    #[test]
    fn relay_is_optional_and_never_authority() {
        let relay = RouteCandidate {
            bearer: Bearer::ManagedRelay,
            ..TRUSTED
        };
        assert_eq!(
            choose_route(PayloadClass::PublicDemo, &[relay]),
            RouteDecision::SimulationOnlyCandidate(Bearer::ManagedRelay)
        );
        assert_eq!(
            choose_route(
                PayloadClass::PublicDemo,
                &[RouteCandidate {
                    dataset_authorized: false,
                    ..relay
                }]
            ),
            RouteDecision::HoldOffline
        );
    }

    #[test]
    fn radio_and_protected_data_are_not_selected() {
        let radio = RouteCandidate {
            bearer: Bearer::LowBandwidthRadio,
            ..TRUSTED
        };
        assert_eq!(
            choose_route(PayloadClass::PublicDemo, &[radio]),
            RouteDecision::HoldOffline
        );
        for class in [
            PayloadClass::Internal,
            PayloadClass::Restricted,
            PayloadClass::Regulated,
        ] {
            assert_eq!(choose_route(class, &[TRUSTED]), RouteDecision::HoldOffline);
        }
    }
}
