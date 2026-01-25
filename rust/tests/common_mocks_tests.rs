mod common;
use common::mocks::InMemoryObjectRepository;
use common::builders::ObjectBuilder;
use just_storage::application::ports::ObjectRepository; // bring trait into scope for methods

#[tokio::test]
async fn test_inmemory_object_repo_save_and_find() {
    let repo = InMemoryObjectRepository::new();
    let obj = ObjectBuilder::new().build();

    repo.save(&obj).await.expect("save failed");

    let found = repo.find_by_id(obj.id()).await.expect("find failed");
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id().to_string(), obj.id().to_string());
}
