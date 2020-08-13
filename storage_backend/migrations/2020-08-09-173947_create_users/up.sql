-- Your SQL goes here

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    online BOOL NOT NULL,
    last_login TIMESTAMP ,
    date_created TIMESTAMP NOT NULL
);