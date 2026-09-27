// Ainux OSI 7-Layer Architecture

pub mod layer1_physical;
pub mod layer2_datalink;
pub mod layer3_network;
pub mod layer4_transport;
pub mod layer5_session;
pub mod layer6_presentation;
pub mod layer7_application;

pub fn init() {
    crate::println!("Initializing OSI 7-Layer Network Stack...");
    layer1_physical::init();
    layer2_datalink::init();
    layer3_network::init();
    layer4_transport::init();
    layer5_session::init();
    layer6_presentation::init();
    layer7_application::init();
    crate::println!("OSI Network Stack Initialized.");
}
