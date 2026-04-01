use one_sec::{api::Endpoint, icp, task::TaskType};

use crate::TestEnv;

#[tokio::test]
async fn test_wai_per_icp_rate() {
    let test = TestEnv::new().await;
    let rate = test.get_wei_per_icp_rate().await;
    assert_eq!(rate, 0.0);
    for _ in 0..40 {
        test.tick().await;
    }
    // Now the rate should be updated.
    let rate = test.get_wei_per_icp_rate().await;
    assert_eq!(rate, 25338360.0);
}

#[tokio::test]
async fn test_controller_only_endpoints() {
    let test = TestEnv::new().await;

    let task = TaskType::Icp(icp::Task::InitializeEcdsaPublicKey);
    let result = test.pause_task(test.user, task).await;
    assert_eq!(
        result,
        "Only a controller or a relayer can call this endpoint"
    );
    let result = test.resume_task(test.user, task).await;
    assert_eq!(
        result,
        "Only a controller or a relayer can call this endpoint"
    );
    let result = test.run_task(test.user, task).await;
    assert_eq!(
        result,
        "Only a controller or a relayer can call this endpoint"
    );
    let result = test.resume_all_paused_tasks(test.user).await;
    assert_eq!(
        result,
        "Only a controller or a relayer can call this endpoint"
    );
    let result = test.pause_task(test.controller, task).await;
    assert_eq!(result, "Ok");
    let result = test.resume_task(test.controller, task).await;
    assert_eq!(result, "Ok");
    let result = test.run_task(test.controller, task).await;
    assert_eq!(result, "Ok");
    let result = test.resume_all_paused_tasks(test.controller).await;
    assert_eq!(result, "Ok");

    let endpoint = Endpoint::Transfer;
    let result = test.pause_endpoint(test.user, endpoint).await;
    assert_eq!(
        result,
        "Only a controller or a relayer can call this endpoint"
    );
    let result = test.resume_endpoint(test.user, endpoint).await;
    assert_eq!(
        result,
        "Only a controller or a relayer can call this endpoint"
    );
    let result = test.resume_all_paused_endpoints(test.user).await;
    assert_eq!(
        result,
        "Only a controller or a relayer can call this endpoint"
    );
    let result = test.pause_endpoint(test.controller, endpoint).await;
    assert_eq!(result, "Ok");
    let result = test.resume_endpoint(test.controller, endpoint).await;
    assert_eq!(result, "Ok");
    let result = test.resume_all_paused_endpoints(test.controller).await;
    assert_eq!(result, "Ok");
}
