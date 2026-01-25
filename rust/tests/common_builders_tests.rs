use std::sync::Arc;
use std::io::Cursor;

use just_storage::application::use_cases::UploadObjectUseCase;
use just_storage::domain::value_objects::StorageClass;

mod common;
use common::TestEnvironment;
use common::builders::{ObjectBuilder, BlobBuilder, UploadRequestBuilder};

#[tokio::test]
async fn test_object_blob_upload_request_builders() {
    // Smoke test builders
    let _obj = ObjectBuilder::new().namespace("x").build();
    let _blob = BlobBuilder::new().build();
    let _req = UploadRequestBuilder::new().namespace("x").build();

    // Ensure builder wiring with use-cases works
    let env = TestEnvironment::builder().with_database(true).with_use_cases(true).build().await;
    let upload_uc = env.upload_use_case.expect("upload use case should be wired");

    // Use the upload use-case with a small reader
    let reader = Box::pin(Cursor::new(b"hello" as &[u8]));
    let req = UploadRequestBuilder::new().build();

    let res = upload_uc.execute(req, reader).await;
    assert!(res.is_ok());
}
