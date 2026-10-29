//! A no-network, no-device reference package using the universal socket SDK.
//! Run: cargo run --locked -p rust-boy-adapter-sdk --example hello_adapter

use rust_boy_adapter_sdk::{
    AdapterCommand, AdapterEnvelope, AdapterError, AdapterReply, UniversalPackageAdapter, Viewport,
    ADAPTER_PROTOCOL,
};
use serde_json::json;

struct HelloPackage {
    id: &'static str,
    active: bool,
}

impl UniversalPackageAdapter for HelloPackage {
    fn package_id(&self) -> &str {
        self.id
    }

    fn handle(&mut self, request: AdapterEnvelope) -> Result<AdapterReply, AdapterError> {
        if request.protocol != ADAPTER_PROTOCOL || request.package_id != self.id {
            return Err(AdapterError::Denied("incompatible package socket"));
        }
        match request.body {
            AdapterCommand::Activate { .. } | AdapterCommand::Resume { .. } => {
                self.active = true;
                Ok(AdapterReply::View {
                    state: json!({"label": "HELLO", "active": self.active}),
                })
            }
            AdapterCommand::Suspend | AdapterCommand::Shutdown => {
                self.active = false;
                Ok(AdapterReply::Ack)
            }
            AdapterCommand::Resize { .. } => Ok(AdapterReply::Ack),
            AdapterCommand::Intent { action, .. } if action == "ui.select" => Ok(AdapterReply::Ack),
            AdapterCommand::Intent { .. } => Err(AdapterError::Denied("unreviewed package intent")),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut package = HelloPackage {
        id: "org.example.hello",
        active: false,
    };
    let request = AdapterEnvelope {
        protocol: ADAPTER_PROTOCOL,
        package_id: package.package_id().into(),
        request_id: 1,
        body: AdapterCommand::Activate {
            viewport: Viewport::new(480, 320)?,
        },
    };
    // Demonstrates the adapter contract only; no host installation, UI
    // rendering, network discovery, audio or physical device connection.
    let reply = package.handle(AdapterEnvelope::decode(&request.encode()?)?)?;
    println!("{}", serde_json::to_string_pretty(&reply)?);
    Ok(())
}
