-- Your SQL goes here



CREATE TABLE interaction_history (
    id BIGSERIAL PRIMARY KEY,
    person BIGSERIAL REFERENCES users(id),
    enjoyed_interaction BOOL,
    start_date TIMESTAMP NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_date TIMESTAMP ,
    end_time TIMESTAMP ,
    mode game_mode NOT NULL
);