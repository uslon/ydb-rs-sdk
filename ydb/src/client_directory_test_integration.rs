use std::time;
use std::time::UNIX_EPOCH;

use tracing_test::traced_test;

use crate::errors::YdbResult;
use crate::test_integration_helper::create_client;

#[tokio::test]
#[traced_test]
#[ignore] // need YDB access
async fn create_list_remove_directory() -> YdbResult<()> {
    let client = create_client().await?;
    let mut scheme_client = client.directory_client();
    let time_now = time::SystemTime::now().duration_since(UNIX_EPOCH)?;
    let directory_name = format!("directory_{}", time_now.as_millis());
    let directory_path = format!("local/{}", directory_name.clone());

    scheme_client.make_directory(directory_path.clone()).await?;
    let directories = scheme_client.list_directory("local/".to_owned()).await?;
    assert!(directories.iter().any(|d| d.name == directory_name));

    scheme_client.remove_directory(directory_path).await?;
    let directories = scheme_client.list_directory("local/".to_owned()).await?;
    assert!(!directories.iter().any(|d| d.name == directory_name));

    Ok(())
}
