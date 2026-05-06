#[cfg(test)]
mod tests {
    use crate::wasm::*;
    use crate::fs::vfs::{MockInode, MockHandle};
    use alloc::sync::Arc;

    /// TEST: Inode-Rooted Confinement (Structural Escape Prevention)
    #[test]
    fn test_structural_confinement() {
        let root = Arc::new(MockInode::new(1, "root"));
        let sub = Arc::new(MockInode::new_with_parent(2, "sub", root.clone()));
        let secret = Arc::new(MockInode::new(3, "secret")); // Not a descendant

        // 1. Valid access
        assert!(sub.is_descendant_of(root.clone()));
        
        // 2. Escape attempt (structural)
        assert!(!secret.is_descendant_of(root.clone()));
        
        // 3. Upward escape attempt
        let fake_parent = Arc::new(MockInode::new(4, "fake"));
        let trapped = Arc::new(MockInode::new_with_parent(5, "trapped", fake_parent));
        assert!(!trapped.is_descendant_of(root.clone()));
    }

    /// TEST: Scoped Registry (Discovery Authority Enforcement)
    #[test]
    fn test_scoped_discovery() {
        let current_global = GLOBAL_EPOCH.load(Ordering::SeqCst);
        let registry_cap = Capability {
            obj: Object::Registry,
            rights: Rights(Rights::LOOKUP),
            global_epoch: current_global,
            resource_id: 1,
            resource_epoch: get_resource_epoch(1),
        };
        
        let console_cap = Capability {
            obj: Object::Console,
            rights: Rights(Rights::WRITE),
            global_epoch: current_global,
            resource_id: 0,
            resource_epoch: get_resource_epoch(0),
        };
        
        register_service(10, "test-service", console_cap.clone());
        
        // Discovery with capability succeeds
        assert!(lookup_service("test-service").is_some());
        
        // Internal Check: register_service correctly tags PID
        assert_eq!(*SERVICE_OWNERSHIP.lock().get("test-service").unwrap(), 10);
    }

    /// TEST: Registry Cleanup (Service Self-Healing)
    #[test]
    fn test_service_cleanup() {
        let current_global = GLOBAL_EPOCH.load(Ordering::SeqCst);
        let cap = Capability {
            obj: Object::Console,
            rights: Rights(Rights::WRITE),
            global_epoch: current_global,
            resource_id: 0,
            resource_epoch: get_resource_epoch(0),
        };
        
        register_service(42, "service-42", cap.clone());
        register_service(43, "service-43", cap.clone());
        
        assert!(lookup_service("service-42").is_some());
        
        // PID 42 dies
        cleanup_services_for_pid(42);
        
        assert!(lookup_service("service-42").is_none()); // Removed
        assert!(lookup_service("service-43").is_some()); // Preserved
    }

    /// TEST: Layered Revocation (Epoch Validation)
    #[test]
    fn test_layered_revocation() {
        let res_id = 5;
        let start_epoch = get_resource_epoch(res_id);
        
        let cap = Capability {
            obj: Object::Console,
            rights: Rights(Rights::WRITE),
            global_epoch: GLOBAL_EPOCH.load(Ordering::SeqCst),
            resource_id: res_id,
            resource_epoch: start_epoch,
        };
        
        // 1. Initial validity
        assert_eq!(cap.resource_epoch, get_resource_epoch(res_id));
        
        // 2. Revoke specific resource
        revoke_resource(res_id);
        
        // 3. Check invalidation
        assert!(cap.resource_epoch < get_resource_epoch(res_id));
    }
}
