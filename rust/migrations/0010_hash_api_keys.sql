-- Add pgcrypto extension for hashing existing plaintext API keys
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Hash existing plaintext API keys
-- We use sha256 to match the application's hashing logic.
UPDATE api_keys 
SET api_key = encode(digest(api_key, 'sha256'), 'hex');

-- The column 'api_key' now contains the SHA-256 hash in hexadecimal format.
-- (No column renaming is strictly necessary, but we understand 'api_key' now stores the hash)
