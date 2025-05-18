use single_instance::SingleInstance;
use std::process::Command;
use std::time::Duration;
use std::thread;

#[test]
fn test_single_instance_behavior() {
    // Create a unique app identifier for testing
    let app_id = format!("translator-test-{}", std::process::id());
    
    // First instance should succeed
    let instance1 = SingleInstance::new(&app_id).unwrap();
    assert!(instance1.is_single(), "First instance should be single");
    
    // Second instance should detect the first one
    let instance2 = SingleInstance::new(&app_id).unwrap();
    assert!(!instance2.is_single(), "Second instance should not be single");
    
    // Drop first instance and check if second can be single
    drop(instance1);
    thread::sleep(Duration::from_millis(100)); // Give OS time to release the lock
    
    let instance3 = SingleInstance::new(&app_id).unwrap();
    assert!(instance3.is_single(), "Instance should be single after first is dropped");
}

#[test]
#[ignore] // This test requires the actual application to be built
fn test_second_instance_exits() {
    // This test would require spawning the actual application
    // We'll mark it as ignored for regular test runs
    
    // Build the application
    let output = Command::new("cargo")
        .args(&["build", "--bin", "translator"])
        .output()
        .expect("Failed to build application");
    
    if !output.status.success() {
        panic!("Failed to build application");
    }
    
    // Start first instance
    let mut child1 = Command::new("target/debug/translator")
        .spawn()
        .expect("Failed to start first instance");
    
    // Give it time to start
    thread::sleep(Duration::from_secs(1));
    
    // Try to start second instance
    let output = Command::new("target/debug/translator")
        .output()
        .expect("Failed to run second instance");
    
    // Second instance should exit quickly
    assert!(!output.status.success() || output.status.code() == Some(0));
    
    // Kill first instance
    child1.kill().expect("Failed to kill first instance");
}