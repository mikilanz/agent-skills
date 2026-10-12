-- gen_random_uuid() is built in from PostgreSQL 13; the extension keeps older servers working.
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
