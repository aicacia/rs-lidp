DROP INDEX IF EXISTS "idx_oauth2_user_consents_user_client";
DROP TABLE IF EXISTS "oauth2_user_consents";

DROP INDEX IF EXISTS "idx_oauth2_authorization_codes_expires";
DROP INDEX IF EXISTS "idx_oauth2_authorization_codes_key_id";
DROP INDEX IF EXISTS "idx_oauth2_authorization_codes_client";
DROP TABLE IF EXISTS "oauth2_authorization_codes";

DROP INDEX IF EXISTS "idx_keys_parent_id";
DROP INDEX IF EXISTS "idx_keys_entity_type_entity_id";
DROP INDEX IF EXISTS "idx_keys_derivation_path";
DROP TABLE IF EXISTS "keys";

DROP INDEX IF EXISTS "idx_clients_client_name";
DROP TABLE IF EXISTS "clients";

DROP INDEX IF EXISTS "idx_user_passwords_user_id";
DROP TABLE IF EXISTS "user_passwords";

DROP INDEX IF EXISTS "idx_user_phone_numbers_primary";
DROP INDEX IF EXISTS "idx_user_phone_numbers_user_id";
DROP TABLE IF EXISTS "user_phone_numbers";

DROP INDEX IF EXISTS "idx_user_emails_primary";
DROP INDEX IF EXISTS "idx_user_emails_user_id";
DROP TABLE IF EXISTS "user_emails";

DROP INDEX IF EXISTS "idx_users_name";
DROP TABLE IF EXISTS "users";
