-- Your SQL goes here

CREATE TABLE abstract_personality_schema (
    user_id BIGSERIAL REFERENCES users(id) PRIMARY KEY,
    binary_encoding TEXT,
    last_systematic_update TIMESTAMP,
    last_micro_update TIMESTAMP
);