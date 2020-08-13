-- Your SQL goes here
-- this directory was made hidden so that the game_mode_option_table can be setup first!


CREATE TABLE interaction_history (
    id BIGSERIAL PRIMARY KEY,
    person BIGSERIAL REFERENCES users(id),
    enjoyed_interaction BOOL,
    start_date TIMESTAMP NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_date TIMESTAMP ,
    end_time TIMESTAMP ,
    mode text REFERENCES game_mode (valid_mode) ON UPDATE CASCADE
);