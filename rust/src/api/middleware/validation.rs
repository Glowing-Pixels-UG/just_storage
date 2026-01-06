use axum::http::StatusCode;
use validator::Validate;

/// Validation error response
#[derive(serde::Serialize)]
pub struct ValidationErrorResponse {
    pub error: String,
    pub field_errors: Vec<FieldError>,
}

/// Field error details
#[derive(serde::Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// Validate a payload and return a proper API error response
pub fn validate_and_respond<T>(payload: &T) -> Result<(), (StatusCode, ValidationErrorResponse)>
where
    T: Validate,
{
    payload.validate().map_err(|validation_errors| {
        let field_errors = validation_errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(|error| FieldError {
                    field: field.to_string(),
                    message: error
                        .message
                        .as_ref()
                        .map(|cow| cow.to_string())
                        .unwrap_or_else(|| "Invalid value".to_string()),
                })
            })
            .collect();

        let response = ValidationErrorResponse {
            error: "Validation failed".to_string(),
            field_errors,
        };

        (StatusCode::UNPROCESSABLE_ENTITY, response)
    })
}
