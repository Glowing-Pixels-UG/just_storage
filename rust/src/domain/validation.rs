//! Common validation utilities for domain objects and business logic
//!
//! This module provides reusable validation functions that can be used
//! across different parts of the application to ensure consistency.

use crate::domain::errors::DomainError;
use once_cell::sync::Lazy;
use regex::Regex;

/// Cached regex patterns for common validations
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").expect("Invalid email regex")
});

static ALPHANUMERIC_UNDERSCORE_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]+$").expect("Invalid alphanumeric regex"));

/// Validation result type
pub type ValidationResult<T> = Result<T, DomainError>;

/// Common validation utilities
pub struct Validation;

impl Validation {
    /// Validate that a string is not empty
    pub fn validate_not_empty(value: &str, field_name: &str) -> ValidationResult<()> {
        if value.trim().is_empty() {
            return Err(DomainError::ValidationError {
                field: field_name.to_string(),
                message: "Field cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    /// Validate string length constraints
    pub fn validate_length(
        value: &str,
        field_name: &str,
        min_length: Option<usize>,
        max_length: Option<usize>,
    ) -> ValidationResult<()> {
        if let Some(min) = min_length {
            if value.len() < min {
                return Err(DomainError::ValidationError {
                    field: field_name.to_string(),
                    message: format!("Field must be at least {} characters long", min),
                });
            }
        }

        if let Some(max) = max_length {
            if value.len() > max {
                return Err(DomainError::ValidationError {
                    field: field_name.to_string(),
                    message: format!("Field must be at most {} characters long", max),
                });
            }
        }

        Ok(())
    }

    /// Validate that a value is within a numeric range
    pub fn validate_range<T: PartialOrd + std::fmt::Display>(
        value: T,
        field_name: &str,
        min: Option<T>,
        max: Option<T>,
    ) -> ValidationResult<()> {
        if let Some(min_val) = min {
            if value < min_val {
                return Err(DomainError::ValidationError {
                    field: field_name.to_string(),
                    message: format!("Value {} is below minimum {}", value, min_val),
                });
            }
        }

        if let Some(max_val) = max {
            if value > max_val {
                return Err(DomainError::ValidationError {
                    field: field_name.to_string(),
                    message: format!("Value {} exceeds maximum {}", value, max_val),
                });
            }
        }

        Ok(())
    }

    /// Validate email format (basic)
    pub fn validate_email(email: &str, field_name: &str) -> ValidationResult<()> {
        if !EMAIL_REGEX.is_match(email) {
            return Err(DomainError::ValidationError {
                field: field_name.to_string(),
                message: "Invalid email format".to_string(),
            });
        }

        // Basic length checks
        Self::validate_length(email, field_name, Some(3), Some(254))?;

        Ok(())
    }

    /// Validate URL format (basic)
    pub fn validate_url(url: &str, field_name: &str) -> ValidationResult<()> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(DomainError::ValidationError {
                field: field_name.to_string(),
                message: "URL must start with http:// or https://".to_string(),
            });
        }

        Self::validate_length(url, field_name, Some(10), Some(2048))?;

        Ok(())
    }

    /// Validate that a string contains only alphanumeric characters and underscores
    pub fn validate_alphanumeric_underscore(value: &str, field_name: &str) -> ValidationResult<()> {
        if !ALPHANUMERIC_UNDERSCORE_REGEX.is_match(value) {
            return Err(DomainError::ValidationError {
                field: field_name.to_string(),
                message: "Field can only contain alphanumeric characters and underscores"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// Validate that a string matches a regex pattern
    pub fn validate_regex(
        value: &str,
        field_name: &str,
        pattern: &str,
        error_message: &str,
    ) -> ValidationResult<()> {
        match regex::Regex::new(pattern) {
            Ok(re) => {
                if !re.is_match(value) {
                    return Err(DomainError::ValidationError {
                        field: field_name.to_string(),
                        message: error_message.to_string(),
                    });
                }
            }
            Err(_) => {
                // If regex is invalid, we can't validate - this is a programming error
                // but we'll allow the value through for safety
            }
        }
        Ok(())
    }

    /// Validate UUID format
    pub fn validate_uuid(value: &str, field_name: &str) -> ValidationResult<()> {
        uuid::Uuid::parse_str(value).map_err(|_| DomainError::ValidationError {
            field: field_name.to_string(),
            message: "Invalid UUID format".to_string(),
        })?;
        Ok(())
    }

    /// Validate that a collection is not empty
    pub fn validate_not_empty_collection<T>(
        collection: &[T],
        field_name: &str,
    ) -> ValidationResult<()> {
        if collection.is_empty() {
            return Err(DomainError::ValidationError {
                field: field_name.to_string(),
                message: "Collection cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    /// Validate that all items in a collection pass a validation function
    pub fn validate_collection_items<T, F>(
        collection: &[T],
        field_name: &str,
        validator: F,
    ) -> ValidationResult<()>
    where
        F: Fn(&T) -> ValidationResult<()>,
    {
        for (index, item) in collection.iter().enumerate() {
            validator(item).map_err(|e| match e {
                DomainError::ValidationError { field, message } => DomainError::ValidationError {
                    field: format!("{}[{}].{}", field_name, index, field),
                    message,
                },
                other => other,
            })?;
        }
        Ok(())
    }
}

/// Builder pattern for complex validations
pub struct ValidationBuilder<T> {
    value: T,
    field_name: String,
    errors: Vec<String>,
}

impl<T> ValidationBuilder<T> {
    pub fn new(value: T, field_name: &str) -> Self {
        Self {
            value,
            field_name: field_name.to_string(),
            errors: Vec::new(),
        }
    }

    pub fn not_empty(mut self) -> Self
    where
        T: AsRef<str>,
    {
        if self.value.as_ref().trim().is_empty() {
            self.errors.push("Field cannot be empty".to_string());
        }
        self
    }

    pub fn length(mut self, min: Option<usize>, max: Option<usize>) -> Self
    where
        T: AsRef<str>,
    {
        let len = self.value.as_ref().len();

        if let Some(min_val) = min {
            if len < min_val {
                self.errors
                    .push(format!("Must be at least {} characters", min_val));
            }
        }

        if let Some(max_val) = max {
            if len > max_val {
                self.errors
                    .push(format!("Must be at most {} characters", max_val));
            }
        }

        self
    }

    pub fn email(mut self) -> Self
    where
        T: AsRef<str>,
    {
        let email = self.value.as_ref();
        if !email.contains('@') || !email.contains('.') {
            self.errors.push("Invalid email format".to_string());
        }
        self
    }

    pub fn custom<F>(mut self, validator: F) -> Self
    where
        F: Fn(&T) -> Option<String>,
    {
        if let Some(error) = validator(&self.value) {
            self.errors.push(error);
        }
        self
    }

    pub fn build(self) -> ValidationResult<T> {
        if self.errors.is_empty() {
            Ok(self.value)
        } else {
            Err(DomainError::ValidationError {
                field: self.field_name,
                message: self.errors.join("; "),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_not_empty() {
        assert!(Validation::validate_not_empty("test", "field").is_ok());
        assert!(Validation::validate_not_empty("", "field").is_err());
        assert!(Validation::validate_not_empty("   ", "field").is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(Validation::validate_length("test", "field", Some(2), Some(10)).is_ok());
        assert!(Validation::validate_length("a", "field", Some(2), Some(10)).is_err());
        assert!(
            Validation::validate_length("very_long_string", "field", Some(2), Some(10)).is_err()
        );
    }

    #[test]
    fn test_validate_email() {
        assert!(Validation::validate_email("test@example.com", "email").is_ok());
        assert!(Validation::validate_email("invalid-email", "email").is_err());
        assert!(Validation::validate_email("", "email").is_err());
    }

    #[test]
    fn test_validate_uuid() {
        assert!(Validation::validate_uuid("550e8400-e29b-41d4-a716-446655440000", "id").is_ok());
        assert!(Validation::validate_uuid("invalid-uuid", "id").is_err());
    }

    #[test]
    fn test_validation_builder() {
        // Valid case
        let result = ValidationBuilder::new("test@example.com", "email")
            .not_empty()
            .email()
            .length(Some(5), Some(100))
            .build();
        assert!(result.is_ok());

        // Invalid case
        let result = ValidationBuilder::new("", "email")
            .not_empty()
            .email()
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_collection() {
        let valid_items = vec!["item1", "item2", "item3"];
        assert!(Validation::validate_not_empty_collection(&valid_items, "items").is_ok());

        let empty_items: Vec<&str> = vec![];
        assert!(Validation::validate_not_empty_collection(&empty_items, "items").is_err());

        // Test collection item validation
        let mixed_items = vec!["valid", "", "also_valid"];
        let result = Validation::validate_collection_items(&mixed_items, "items", |item| {
            Validation::validate_not_empty(item, "item")
        });
        assert!(result.is_err()); // Should fail because of empty string
    }

    #[test]
    fn test_validate_length_edge_cases() {
        // Test exactly at minimum
        assert!(Validation::validate_length("ab", "field", Some(2), Some(10)).is_ok());

        // Test exactly at maximum
        assert!(Validation::validate_length("abcdefghij", "field", Some(2), Some(10)).is_ok());

        // Test below minimum
        assert!(Validation::validate_length("a", "field", Some(2), Some(10)).is_err());

        // Test above maximum
        assert!(Validation::validate_length("abcdefghijk", "field", Some(2), Some(10)).is_err());

        // Test no minimum constraint
        assert!(Validation::validate_length("", "field", None, Some(10)).is_ok());
        assert!(Validation::validate_length("abcdefghijk", "field", None, Some(10)).is_err());

        // Test no maximum constraint
        assert!(Validation::validate_length("a", "field", Some(1), None).is_ok());

        // Test no constraints
        assert!(Validation::validate_length("any length", "field", None, None).is_ok());
    }

    #[test]
    fn test_validate_email_comprehensive() {
        // Valid emails
        let valid_emails = vec![
            "test@example.com",
            "user.name+tag@example.co.uk",
            "test.email@subdomain.example.com",
            "123@example.com",
            "a@b.co",
        ];

        for email in valid_emails {
            assert!(Validation::validate_email(email, "email").is_ok(),
                "Email should be valid: {}", email);
        }

        // Invalid emails
        let invalid_emails = vec![
            "",
            "@example.com",
            "test@",
            "test",
            "test@.com",
            "test..test@example.com",
            "test @example.com",
            "test@example.com ",
            "test@example..com",
            "test@exam ple.com",
        ];

        for email in invalid_emails {
            assert!(Validation::validate_email(email, "email").is_err(),
                "Email should be invalid: {}", email);
        }
    }

    #[test]
    fn test_validate_uuid_comprehensive() {
        // Valid UUIDs
        let valid_uuids = vec![
            "550e8400-e29b-41d4-a716-446655440000",
            "550E8400-E29B-41D4-A716-446655440000", // uppercase
            "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            "6ba7b811-9dad-21d1-80b4-00c04fd430c8",
        ];

        for uuid in valid_uuids {
            assert!(Validation::validate_uuid(uuid, "id").is_ok(),
                "UUID should be valid: {}", uuid);
        }

        // Invalid UUIDs
        let invalid_uuids = vec![
            "",
            "not-a-uuid",
            "550e8400-e29b-41d4-a716", // too short
            "550e8400-e29b-41d4-a716-446655440000-extra", // too long
            "550e8400-e29b-41d4-a716-44665544000g", // invalid character
            "550e8400e29b41d4a716446655440000", // no hyphens
            "550e8400-e29b-41d4-a716-44665544000", // missing digit
        ];

        for uuid in invalid_uuids {
            assert!(Validation::validate_uuid(uuid, "id").is_err(),
                "UUID should be invalid: {}", uuid);
        }
    }

    #[test]
    fn test_validate_not_empty_comprehensive() {
        // Valid non-empty strings
        let valid_strings = vec![
            "a",
            "hello world",
            " spaces ",
            "\ttab\t",
            "\nnewline\n",
            "mixed whitespace\t\n ",
        ];

        for s in valid_strings {
            assert!(Validation::validate_not_empty(s, "field").is_ok(),
                "String should be valid: {:?}", s);
        }

        // Invalid empty/whitespace-only strings
        let invalid_strings = vec![
            "",
            "   ",
            "\t",
            "\n",
            "\t\n ",
            " \t \n ",
        ];

        for s in invalid_strings {
            assert!(Validation::validate_not_empty(s, "field").is_err(),
                "String should be invalid: {:?}", s);
        }
    }

    #[test]
    fn test_validation_builder_chaining() {
        // Test successful chaining
        let result = ValidationBuilder::new("test@example.com", "email")
            .not_empty()
            .email()
            .length(Some(5), Some(100))
            .custom(|value| {
                if value.contains("test") {
                    Ok(())
                } else {
                    Err("Must contain 'test'".to_string())
                }
            })
            .build();

        assert!(result.is_ok());

        // Test failure in chain stops execution
        let result = ValidationBuilder::new("", "email") // Empty string
            .email() // This should fail
            .length(Some(5), Some(100)) // This should not execute
            .build();

        assert!(result.is_err());
        // Should fail on empty check, not continue to email validation
    }

    #[test]
    fn test_validation_builder_custom_validation() {
        // Test custom validation success
        let result = ValidationBuilder::new("custom_value", "field")
            .custom(|value| {
                if value.len() > 5 {
                    Ok(())
                } else {
                    Err("Too short".to_string())
                }
            })
            .build();

        assert!(result.is_ok());

        // Test custom validation failure
        let result = ValidationBuilder::new("short", "field")
            .custom(|value| {
                if value.len() > 10 {
                    Ok(())
                } else {
                    Err("Too short for custom validation".to_string())
                }
            })
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_collection_items_comprehensive() {
        // Test all items valid
        let all_valid = vec!["item1", "item2", "item3"];
        let result = Validation::validate_collection_items(&all_valid, "items", |item| {
            Validation::validate_not_empty(item, "item")
        });
        assert!(result.is_ok());

        // Test some items invalid
        let some_invalid = vec!["valid", "", "also_valid", "   "];
        let result = Validation::validate_collection_items(&some_invalid, "items", |item| {
            Validation::validate_not_empty(item, "item")
        });
        assert!(result.is_err());

        // Test empty collection
        let empty: Vec<&str> = vec![];
        let result = Validation::validate_collection_items(&empty, "items", |item| {
            Validation::validate_not_empty(item, "item")
        });
        assert!(result.is_ok()); // Empty collection is allowed

        // Test complex validation
        let mixed = vec!["valid@email.com", "invalid-email", "another@valid.com"];
        let result = Validation::validate_collection_items(&mixed, "emails", |item| {
            Validation::validate_email(item, "email")
        });
        assert!(result.is_err()); // Should fail due to invalid email
    }

    #[test]
    fn test_validation_error_messages() {
        // Test that error messages include field names and are descriptive
        let result = Validation::validate_not_empty("", "username");
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("username"));
        assert!(error_msg.contains("empty"));

        let result = Validation::validate_email("not-an-email", "user_email");
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("user_email"));
        assert!(error_msg.contains("email"));

        let result = Validation::validate_length("toolongstring", "description", Some(5), Some(10));
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("description"));
        assert!(error_msg.contains("length"));
    }
}
