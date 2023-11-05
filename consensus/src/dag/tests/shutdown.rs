use crate::dag::shutdown::ShutdownGroup;

#[tokio::test]
async fn test_shutdown() {
    let shutdown_handle = ShutdownGroup::new();
    let (child_handle1, mut child_shutdown1) = shutdown_handle.new_child();
    let (_child_handle2, mut child_shutdown2) = shutdown_handle.new_child();

    let handle1 = tokio::spawn(async move {
        child_shutdown1.recv().await;
    });

    let handle2 = tokio::spawn(async move {
        child_shutdown2.recv().await;
    });

    child_handle1.shutdown().await;
    let _ = handle1.await;

    shutdown_handle.shutdown().await;
    let _ = handle2.await;
}
