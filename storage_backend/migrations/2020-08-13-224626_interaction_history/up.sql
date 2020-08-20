-- Your SQL goes here


CREATE TABLE interaction_history (
    id BIGSERIAL,
    user_id BIGSERIAL REFERENCES users(id),
    enjoyed_interaction BOOL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP ,
    mode text REFERENCES game_modes (valid_mode) ON UPDATE CASCADE NOT NULL,
    PRIMARY KEY(id, user_id)
);